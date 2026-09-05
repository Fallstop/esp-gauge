import { mount } from 'svelte';
import '@fontsource-variable/manrope';
import '@fontsource/ibm-plex-mono/400.css';
import '@fontsource/ibm-plex-mono/500.css';
import './style.css';
import App from './App.svelte';
mount(App, { target: document.getElementById('app')! });
