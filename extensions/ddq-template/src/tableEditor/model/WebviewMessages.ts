import type { TableModel } from './TableModel';

export type EditorContext = {
  kind: 'tblPart' | 'plainTable' | 'new';
  partCount: number;
  partIndex: number;
  attributesEditable: boolean;
  /** 外側 div のクラス（`.landscape` など）。表示専用で、書き戻しでは触らない。 */
  enclosingClasses: string[];
  warnings: string[];
};

/** 拡張ホスト → Webview */
export type ToWebviewMessage =
  | { type: 'init'; model: TableModel; context: EditorContext }
  | { type: 'applied' }
  | { type: 'error'; message: string };

/** Webview → 拡張ホスト */
export type FromWebviewMessage =
  | { type: 'ready' }
  | { type: 'apply'; model: TableModel }
  | { type: 'close' };
