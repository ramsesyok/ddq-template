import { createRoot } from 'react-dom/client';

import { RevisionEditor } from './RevisionEditor';
import { TagTable } from './TagTable';
import './tagTable.css';

// 画面は 2 つあるが、束ねるのは 1 つ。どちらを描くかは HTML の data-view で決める
// （extension.ts / revision/provider.ts が書く）。
const root = document.getElementById('root');
if (root) {
  const view = document.body.dataset.view;
  createRoot(root).render(view === 'revision' ? <RevisionEditor /> : <TagTable />);
}
