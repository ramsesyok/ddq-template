import { createRoot } from 'react-dom/client';

import { TagTable } from './TagTable';
import './tagTable.css';

const root = document.getElementById('root');
if (root) createRoot(root).render(<TagTable />);
