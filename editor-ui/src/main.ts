// Mounts the editor. Theos UI's tokens and component styles come from their one place in the
// repo (P11 §5.7); Vite serves and bundles them, fonts included.

import '../../design/theos-ui/tokens.css';
import '../../design/theos-ui/components/bundle.css';

import { mount } from 'svelte';

import App from './App.svelte';

const target = document.getElementById('app');
if (target === null) {
  throw new Error('index.html has no #app element');
}

export default mount(App, { target });
