import type { FromEditorMessage } from '../revision/messages';
import type { FromWebviewMessage } from '../tagging/messages';

type VsCodeApi<T> = {
  postMessage(message: T): void;
  getState(): unknown;
  setState(state: unknown): void;
};

declare function acquireVsCodeApi(): VsCodeApi<unknown>;

let api: VsCodeApi<unknown> | undefined;

function acquire(): VsCodeApi<unknown> {
  // acquireVsCodeApi は 1 回しか呼べないので、画面をまたいで使い回す
  if (!api) api = acquireVsCodeApi();
  return api;
}

/** ラベル一覧の画面から拡張へ送る。 */
export function vscode(): VsCodeApi<FromWebviewMessage> {
  return acquire() as VsCodeApi<FromWebviewMessage>;
}

/** 改訂履歴の編集画面から拡張へ送る。 */
export function editorApi(): VsCodeApi<FromEditorMessage> {
  return acquire() as VsCodeApi<FromEditorMessage>;
}
