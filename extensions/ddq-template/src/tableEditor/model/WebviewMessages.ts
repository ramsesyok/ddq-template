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
  /** session: 編集セッションの番号（Apply に付けて返す） */
  | { type: 'init'; session: number; model: TableModel; context: EditorContext }
  | { type: 'applied' }
  | { type: 'error'; message: string };

/** Webview → 拡張ホスト */
export type FromWebviewMessage =
  | { type: 'ready' }
  /** session: その表を受け取った init の番号（開き直した表に古い内容を書かないため） */
  | { type: 'apply'; session: number; model: TableModel }
  | { type: 'close' };
