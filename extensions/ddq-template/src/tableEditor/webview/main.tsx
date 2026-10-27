import { createRoot } from 'react-dom/client';
import { TableEditor } from './TableEditor';
import './tableEditor.css';

const container = document.getElementById('root');
if (container) {
  createRoot(container).render(<TableEditor />);
}
