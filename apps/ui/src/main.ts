import { mount } from "svelte";
import App from "./App.svelte";
import { lang } from "./i18n";

document.documentElement.lang = lang;

const target = document.getElementById("app");
if (target) {
  mount(App, { target });
}
