let counter = 0;

/** Webview 側でセルを識別するためだけの ID。ドキュメントには書き出さない。 */
export function generateTableId(): string {
  counter += 1;
  return `tbl-${Date.now().toString(36)}-${counter}`;
}
