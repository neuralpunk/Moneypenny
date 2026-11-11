import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { initDesktop } from "./lib/desktop.svelte";

void initDesktop();

export default mount(App, { target: document.getElementById("app")! });
