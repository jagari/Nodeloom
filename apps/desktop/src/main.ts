import { mount } from 'svelte';
import './app.css';
import App from './App.svelte';
import AppPage from './components/AppPage.svelte';
import { inDesktopApp } from './lib/app';

// The desktop app's own window shows setup and status; the dashboard is
// what masters serve and what browsers open.
const root = inDesktopApp(window, window.location, import.meta.env.DEV) ? AppPage : App;
mount(root, { target: document.getElementById('app')! });
