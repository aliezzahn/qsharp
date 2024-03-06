// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

// @ts-expect-error can't find typings but it works, whatevs
import * as qviz from "@microsoft/quantum-viz.js";
import { useEffect } from "preact/hooks";

export function CircuitPanel(props: { title: string; circuit: object }) {
  useEffect(() => {
    qviz.draw(
      props.circuit,
      document.getElementById("circuit-container"),
      qviz.STYLES["Default"],
    );
  });

  return (
    <div>
      <div>
        <h1>{props.title}</h1>
      </div>
      <div id="circuit-container"></div>
      <div>
        Tip: you can generate a circuit diagram for any operation that takes
        qubits or arrays of qubits as input.
      </div>
      <div>
        <a
          href="#"
          onClick={() =>
            (document.getElementById("jsonSource")!.hidden =
              !document.getElementById("jsonSource")!.hidden)
          }
        >
          show json
        </a>
        <pre id="jsonSource" hidden={true}>
          {JSON.stringify(props.circuit, null, 2)}
        </pre>
      </div>
    </div>
  );
}
