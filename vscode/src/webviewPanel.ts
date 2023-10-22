// Copyright (c) Microsoft Corporation.
// Licensed under the MIT License.

import {
  QscEventTarget,
  VSDiagnostic,
  getCompilerWorker,
  log,
} from "qsharp-lang";
import {
  commands,
  Disposable,
  ExtensionContext,
  Uri,
  ViewColumn,
  Webview,
  WebviewPanel,
  window,
} from "vscode";
import { isQsharpDocument } from "./common";

const histogramRunTimeoutMs = 1000 * 60 * 5; // 5 minutes

export function registerHistogramCommand(context: ExtensionContext) {
  const compilerWorkerScriptPath = Uri.joinPath(
    context.extensionUri,
    "./out/compilerWorker.js"
  ).toString();

  context.subscriptions.push(
    commands.registerCommand("qsharp-vscode.showHistogram", async () => {
      function resultToLabel(result: string | VSDiagnostic): string {
        if (typeof result !== "string") return "ERROR";
        return result;
      }

      const editor = window.activeTextEditor;
      if (!editor || !isQsharpDocument(editor.document)) {
        throw new Error("The currently active window is not a Q# file");
      }

      QSharpWebViewPanel.render(context.extensionUri);

      // Start the worker, run the code, and send the results to the webview
      const worker = getCompilerWorker(compilerWorkerScriptPath);
      const compilerTimeout = setTimeout(() => {
        worker.terminate(); // TODO: Does the 'terminate' in the finally below error if this happens?
      }, histogramRunTimeoutMs);
      try {
        const code = editor.document.getText();

        // TODO: Get the number of shots to run via a quick pick
        const shots = 50000; // TODO: QuickPick

        const evtTarget = new QscEventTarget(true);
        evtTarget.addEventListener("uiResultsRefresh", () => {
          // TODO: Structure results and send to the webview
          const results = evtTarget.getResults();
          const resultCount = evtTarget.resultCount();
          const buckets = new Map();
          for (let i = 0; i < resultCount; ++i) {
            const key = results[i].result;
            const strKey = resultToLabel(key);
            const newValue = (buckets.get(strKey) || 0) + 1;
            buckets.set(strKey, newValue);
          }
          const message = {
            command: "update",
            buckets: Array.from(buckets.entries()),
          };
          QSharpWebViewPanel.currentPanel?.sendMessage(message);
        });

        await worker.run(code, "", shots, evtTarget);
        clearTimeout(compilerTimeout);
      } catch (e: any) {
        log.error("Codegen error. ", e.toString());
        throw new Error("Run failed");
      } finally {
        worker.terminate();
      }
    })
  );
}

function getUri(webview: Webview, extensionUri: Uri, pathList: string[]) {
  return webview.asWebviewUri(Uri.joinPath(extensionUri, ...pathList));
}

export class QSharpWebViewPanel {
  public static currentPanel: QSharpWebViewPanel | undefined;
  private readonly _panel: WebviewPanel;
  private _disposables: Disposable[] = [];

  private constructor(panel: WebviewPanel, extensionUri: Uri) {
    this._panel = panel;
    this._panel.onDidDispose(() => this.dispose(), null, this._disposables);

    this._panel.webview.html = this._getWebviewContent(
      this._panel.webview,
      extensionUri
    );
    this._setWebviewMessageListener(this._panel.webview);
  }

  private _getWebviewContent(webview: Webview, extensionUri: Uri) {
    const webviewCss = getUri(webview, extensionUri, [
      "resources",
      "webview.css",
    ]);
    const webviewJs = getUri(webview, extensionUri, [
      "out",
      "webview",
      "webview.js",
    ]);

    return /*html*/ `
  <!DOCTYPE html>
  <html lang="en">
    <head>
      <meta charset="UTF-8">
      <meta name="viewport" content="width=device-width, initial-scale=1.0">
      <title>Q#</title>
      <link rel="stylesheet" href="${webviewCss}" />
      <script src="${webviewJs}"></script>
    </head>
    <body>
    </body>
  </html>
`;
  }

  sendMessage(message: any) {
    this._panel.webview.postMessage(message);
  }

  private _setWebviewMessageListener(webview: Webview) {
    webview.onDidReceiveMessage(
      (message: any) => {
        // No messages are currently sent from the webview
        log.debug("Message for webview received", message);
      },
      undefined,
      this._disposables
    );
  }

  public static render(extensionUri: Uri) {
    if (QSharpWebViewPanel.currentPanel) {
      // If the webview panel already exists reveal it
      QSharpWebViewPanel.currentPanel._panel.reveal(ViewColumn.One);
    } else {
      // If a webview panel does not already exist create and show a new one
      const panel = window.createWebviewPanel(
        "qsharpWebView",
        "Q#",
        ViewColumn.Beside,
        {
          enableScripts: true,
        }
      );

      QSharpWebViewPanel.currentPanel = new QSharpWebViewPanel(
        panel,
        extensionUri
      );
    }
  }

  public dispose() {
    QSharpWebViewPanel.currentPanel = undefined;

    // Dispose of the current webview panel
    this._panel.dispose();

    // Dispose of all disposables (i.e. commands) for the current webview panel
    while (this._disposables.length) {
      const disposable = this._disposables.pop();
      if (disposable) {
        disposable.dispose();
      }
    }
  }
}
