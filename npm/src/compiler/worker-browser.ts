// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

import { createWorker } from "../workers/browser.js";
import { compilerProtocol } from "./compiler.js";

function postLogMessage(level: number, target: string, ...args: any) {
  let data = args;
  try {
    structuredClone(args);
  } catch (e) {
    // uncloneable object
    data = ["unsupported log data from worker"];
  }
  self.postMessage({
    messageType: "event",
    type: "log-event",
    detail: { level, target, data },
  });
}

// This export should be assigned to 'self.onmessage' in a WebWorker
export const messageHandler = createWorker(compilerProtocol);
