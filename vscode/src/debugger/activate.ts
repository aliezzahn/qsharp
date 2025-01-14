// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

/* eslint-disable @typescript-eslint/no-unused-vars */

import { IDebugServiceWorker, getDebugServiceWorker, log } from "qsharp-lang";
import { qsharpExtensionId, isQsharpDocument } from "../common";
import { QscDebugSession } from "./session";
import { getRandomGuid } from "../utils";

import * as vscode from "vscode";
import { loadProject } from "../projectSystem";

let debugServiceWorkerFactory: () => IDebugServiceWorker;

export async function activateDebugger(
  context: vscode.ExtensionContext,
): Promise<void> {
  const debugWorkerScriptPath = vscode.Uri.joinPath(
    context.extensionUri,
    "./out/debugger/debug-service-worker.js",
  );

  debugServiceWorkerFactory = () =>
    getDebugServiceWorker(
      debugWorkerScriptPath.toString(),
    ) as IDebugServiceWorker;
  registerCommands(context);

  const provider = new QsDebugConfigProvider();
  context.subscriptions.push(
    vscode.debug.registerDebugConfigurationProvider("qsharp", provider),
  );

  const factory = new InlineDebugAdapterFactory();
  context.subscriptions.push(
    vscode.debug.registerDebugAdapterDescriptorFactory("qsharp", factory),
  );
}

function registerCommands(context: vscode.ExtensionContext) {
  context.subscriptions.push(
    vscode.commands.registerCommand(
      `${qsharpExtensionId}.runEditorContents`,
      (resource: vscode.Uri) =>
        startDebugging(
          resource,
          { name: "Run Q# File", stopOnEntry: false },
          { noDebug: true },
        ),
    ),
    vscode.commands.registerCommand(
      `${qsharpExtensionId}.debugEditorContents`,
      (resource: vscode.Uri) =>
        startDebugging(resource, { name: "Debug Q# File", stopOnEntry: true }),
    ),
    vscode.commands.registerCommand(
      `${qsharpExtensionId}.debugEditorContentsWithCircuit`,
      (resource: vscode.Uri) =>
        startDebugging(resource, {
          name: "Debug Q# File",
          stopOnEntry: true,
          showCircuit: true,
        }),
    ),
  );

  function startDebugging(
    resource: vscode.Uri,
    config: { name: string; [key: string]: any },
    options?: vscode.DebugSessionOptions,
  ) {
    let targetResource = resource;
    if (!targetResource && vscode.window.activeTextEditor) {
      targetResource = vscode.window.activeTextEditor.document.uri;
    }

    if (targetResource) {
      // We'll omit config.program and let the configuration
      // resolver fill it in with the currently open editor's URI.
      // This will also let us correctly handle untitled files
      // where the save prompt pops up before the debugger is launched,
      // potentially causing the active editor URI to change if
      // the file is saved with a different name.
      vscode.debug.startDebugging(
        undefined,
        {
          type: "qsharp",
          request: "launch",
          shots: 1,
          ...config,
        },
        options,
      );
    }
  }
}

class QsDebugConfigProvider implements vscode.DebugConfigurationProvider {
  resolveDebugConfigurationWithSubstitutedVariables(
    folder: vscode.WorkspaceFolder | undefined,
    config: vscode.DebugConfiguration,
    _token?: vscode.CancellationToken | undefined,
  ): vscode.ProviderResult<vscode.DebugConfiguration> {
    // if launch.json is missing or empty
    if (!config.type && !config.request && !config.name) {
      const editor = vscode.window.activeTextEditor;
      if (editor && isQsharpDocument(editor.document)) {
        config.type = "qsharp";
        config.name = "Launch";
        config.request = "launch";
        config.programUri = editor.document.uri.toString();
        config.shots = 1;
        config.noDebug = "noDebug" in config ? config.noDebug : false;
        config.stopOnEntry = !config.noDebug;
      }
    } else if (config.program && folder) {
      // A program is specified in launch.json.
      //
      // Variable substitution is a bit odd in VS Code. Variables such as
      // ${file} and ${workspaceFolder} are expanded to absolute filesystem
      // paths with platform-specific separators. To correctly convert them
      // back to a URI, we need to use the vscode.Uri.file constructor.
      //
      // However, this gives us the URI scheme file:// , which is not correct
      // when the workspace uses a virtual filesystem such as qsharp-vfs://
      // or vscode-test-web://. So now we also need the workspace folder URI
      // to use as the basis for our file URI.
      //
      // Examples of program paths that can come through variable substitution:
      // C:\foo\bar.qs
      // \foo\bar.qs
      // /foo/bar.qs
      const fileUri = vscode.Uri.file(config.program);
      config.programUri = folder.uri
        .with({
          path: fileUri.path,
        })
        .toString();
    } else {
      // Use the active editor if no program is specified.
      const editor = vscode.window.activeTextEditor;
      if (editor && isQsharpDocument(editor.document)) {
        config.programUri = editor.document.uri.toString();
      }
    }

    log.trace(
      `resolveDebugConfigurationWithSubstitutedVariables config.program=${
        config.program
      } folder.uri=${folder?.uri.toString()} config.programUri=${
        config.programUri
      }`,
    );

    if (!config.programUri) {
      // abort launch
      return vscode.window
        .showInformationMessage("Cannot find a Q# program to debug")
        .then((_) => {
          return undefined;
        });
    }
    return config;
  }

  resolveDebugConfiguration(
    folder: vscode.WorkspaceFolder | undefined,
    config: vscode.DebugConfiguration,
    _token?: vscode.CancellationToken | undefined,
  ): vscode.ProviderResult<vscode.DebugConfiguration> {
    // apply defaults if not set
    config.type = config.type ?? "qsharp";
    config.name = config.name ?? "Launch";
    config.request = config.request ?? "launch";
    config.shots = config.shots ?? 1;
    config.entry = config.entry ?? "";
    config.trace = config.trace ?? false;
    // noDebug is set to true when the user runs the program without debugging.
    // otherwise it usually isn't set, but we default to false.
    config.noDebug = config.noDebug ?? false;
    // stopOnEntry is set to true when the user runs the program with debugging.
    // unless overridden.
    config.stopOnEntry = config.stopOnEntry ?? !config.noDebug;

    return config;
  }
}

class InlineDebugAdapterFactory
  implements vscode.DebugAdapterDescriptorFactory
{
  async createDebugAdapterDescriptor(
    session: vscode.DebugSession,
    _executable: vscode.DebugAdapterExecutable | undefined,
  ): Promise<vscode.DebugAdapterDescriptor> {
    const worker = debugServiceWorkerFactory();
    const uri = vscode.Uri.parse(session.configuration.programUri);
    const project = await loadProject(uri);
    const qscSession = new QscDebugSession(
      worker,
      session.configuration,
      project.sources,
      project.languageFeatures,
    );

    await qscSession.init(getRandomGuid());

    return new vscode.DebugAdapterInlineImplementation(qscSession);
  }
}
