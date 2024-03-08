// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// @ts-expect-error can't find typings but it works, whatevs
import * as qviz from "@microsoft/quantum-viz.js";
import { useEffect, useRef } from "preact/hooks";

export function CircuitPanel(props: {
  title: string;
  subtitle: string;
  circuit?: object;
  error?: object;
}) {
  const circuitDiv = useRef<HTMLDivElement>(null);
  const errorPre = useRef<HTMLPreElement>(null);

  useEffect(() => {
    if (props.circuit) {
      qviz.draw(props.circuit, circuitDiv.current, qviz.STYLES["Default"]);
    } else {
      circuitDiv.current!.innerHTML = "";
    }

    if (props.error) {
      errorPre.current!.innerHTML = JSON.stringify(props.error, null, 2);
    } else {
      errorPre.current!.innerHTML = "";
    }
  }, [props.circuit, props.error]);

  return (
    <div>
      <div>
        <h1>{props.title}</h1>
        <h2>{props.subtitle}</h2>
      </div>
      <div ref={circuitDiv}></div>
      <div>
        Tip: you can generate a circuit diagram for any operation that takes
        qubits or arrays of qubits as input.
      </div>
      <div>
        <pre ref={errorPre}></pre>
      </div>
    </div>
  );
}
