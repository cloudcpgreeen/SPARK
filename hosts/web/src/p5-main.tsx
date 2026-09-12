import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import P5App from './P5App';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <P5App />
  </StrictMode>,
);
