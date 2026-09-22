import type { FromWebviewMessage } from '../tagging/messages';

type VsCodeApi = {
  postMessage(message: FromWebviewMessage): void;
  getState(): unknown;
  setState(state: unknown): void;
};

declare function acquireVsCodeApi(): VsCodeApi;

let api: VsCodeApi | undefined;

export function vscode(): VsCodeApi {
  if (!api) api = acquireVsCodeApi();
  return api;
}
