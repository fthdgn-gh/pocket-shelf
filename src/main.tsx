// @title Pocket Shelf
import App from "./app.tsx";
import { scriptDone, setupBegin, timed } from "./diagnostics.ts";
import { mount } from "@pocketjs/framework/solid";

setupBegin();
timed("mount", () => mount(() => <App />));
scriptDone();
