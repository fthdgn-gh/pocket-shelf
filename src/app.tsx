// Pocket Shelf: lists the titles installed on the device and launches them.
//
//   state.ts        selection, theme, view mode, menu, icon loading
//   input.ts        button bindings
//   themes.ts       color palettes
//   text.ts         fonts and the text class per role
//   catalog.ts      reads the host's installed-title table
//   navigation.ts   view modes and D-pad movement per view
//   views/          shelf (carousel), grid and list
//   components/     title art, header, footer, drawers, keyboard

import { onMount, Show } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import { loadCatalog } from "./catalog.ts";
import { TINT_AMBIENT } from "./components/art.tsx";
import { ArtPickerOverlay } from "./components/art-picker.tsx";
import { Backdrop } from "./components/backdrop.tsx";
import { CategoryManagerOverlay } from "./components/category-manager.tsx";
import { EditorOverlay } from "./components/editor.tsx";
import { Footer } from "./components/footer.tsx";
import { Header } from "./components/header.tsx";
import { Prompt } from "./components/icons.tsx";
import { KeyboardOverlay } from "./components/keyboard.tsx";
import { MenuOverlay } from "./components/menu.tsx";
import { OnlineOverlay } from "./components/online.tsx";
import { installInput } from "./input.ts";
import { createLauncherState } from "./state.ts";
import { CarouselView } from "./views/carousel.tsx";
import { GridView } from "./views/grid.tsx";
import { ListView } from "./views/list.tsx";

export default function App() {
  const state = createLauncherState(loadCatalog());

  onMount(() => installInput(state));

  const selected = () => state.games()[state.selectedIndex()];

  return (
    <View
      class="relative flex-col w-full h-full bg-gradient-to-b from-black to-black"
      style={{ gradFrom: state.theme().bgTop, gradTo: state.theme().bgBottom }}
    >
      {/* The selected title's picture, or its tint washed over the top of the screen. */}
      {/* The Dynamic theme already colors the screen from the title, so it has no wash. */}
      <Show when={!state.backdrop() && state.theme().id !== "dynamic" ? selected() : undefined}>
        {(game) => <View class={TINT_AMBIENT[game().tint % TINT_AMBIENT.length]} />}
      </Show>
      <Backdrop state={state} />

      <Header state={state} />

      <Show
        when={state.games().length > 0}
        fallback={
          <View class="flex-col w-full grow items-center justify-center gap-1">
            <Text class={state.text().title} style={{ textColor: state.theme().text }}>
              {state.hasTitles() ? "Nothing in this category yet" : "No installed apps found"}
            </Text>
            <Show when={state.hasTitles()}>
              <Prompt state={state} parts={["Press", { icon: "triangle" }, "on a title to move it here"]} />
            </Show>
          </View>
        }
      >
        {/* Keyed on the category so each category starts from a fresh scroll position. */}
        <Show when={state.categoryId()} keyed>
          <Show when={state.view() === "carousel"}>
            <CarouselView state={state} />
          </Show>
          <Show when={state.view() === "grid"}>
            <GridView state={state} />
          </Show>
          <Show when={state.view() === "list"}>
            <ListView state={state} />
          </Show>
        </Show>
      </Show>

      <Footer state={state} />

      <Show when={state.menuOpen()}>
        <MenuOverlay state={state} />
      </Show>
      <Show when={state.modal() === "categories"}>
        <CategoryManagerOverlay state={state} />
      </Show>
      <Show when={state.editorOpen()}>
        <EditorOverlay state={state} />
      </Show>
      <Show when={state.online.open()}>
        <OnlineOverlay state={state} />
      </Show>
      <Show when={state.modal() === "art"}>
        <ArtPickerOverlay state={state} />
      </Show>
      <Show when={state.modal() === "keyboard"}>
        <KeyboardOverlay state={state} />
      </Show>
    </View>
  );
}
