import { mount } from 'svelte';
import App from './App.svelte';
import { harness } from './harness/bridge.ts';
import Showcase from './harness/Showcase.svelte';
import './harness/styles.css';

// The Playwright suites drive the page through this object (tests/e2e/helpers.ts).
window.__harness = harness;

const target = document.getElementById('app');
if (target === null) throw new Error('index.html has no #app');
const showcase = new URLSearchParams(location.search).get('view') === 'components';
mount(showcase ? Showcase : App, { target });
