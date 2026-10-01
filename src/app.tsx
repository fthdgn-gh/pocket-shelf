// Pocket Shelf: lists the titles installed on the device and launches them.
//
//   state.ts        selection, theme, view mode, menu, icon loading
//   input.ts        button bindings
//   themes.ts       color themes (class literals per UI role)
//   catalog.ts      reads the host's installed-title table
//   navigation.ts   view modes and D-pad movement per view
//   views/          carousel, grid and list
//   components/     title art, header, footer, SELECT menu

import { onMount, Show } from "solid-js";
import { Text, View, type NodeMirror } from "@pocketjs/framework/components";
import { glyph } from "@pocketjs/framework/modality";
import { loadCatalog } from "./catalog.ts";
import { ArtPickerOverlay } from "./components/art-picker.tsx";
import { CategoryManagerOverlay } from "./components/category-manager.tsx";
import { EditorOverlay } from "./components/editor.tsx";
import { Footer } from "./components/footer.tsx";
import { Header } from "./components/header.tsx";
import { KeyboardOverlay } from "./components/keyboard.tsx";
import { MenuOverlay } from "./components/menu.tsx";
import { installInput } from "./input.ts";
import { createLauncherState } from "./state.ts";
import { CarouselView } from "./views/carousel.tsx";
import { GridView } from "./views/grid.tsx";
import { ListView } from "./views/list.tsx";

export default function App() {
  const state = createLauncherState(loadCatalog());

  let rootRef: NodeMirror | undefined;
  onMount(() => {
    if (rootRef) installInput(state, rootRef);
  });

  return (
    <View
      ref={(el) => {
        rootRef = el;
      }}
      class={state.theme().screen}
    >
      <Header state={state} />

      <Show
        when={state.games().length > 0}
        fallback={
          <View class="w-full grow items-center justify-center">
            <Text class={state.theme().empty}>
              {state.hasTitles() ? "Nothing in this category yet" : "No installed apps found"}
            </Text>
            <Show when={state.hasTitles()}>
              <Text class={state.theme().footerDim}>
                {`Use Edit (${glyph("triangle")}) on a title to move it here`}
              </Text>
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
      <Show when={state.modal() === "art"}>
        <ArtPickerOverlay state={state} />
      </Show>
      <Show when={state.modal() === "keyboard"}>
        <KeyboardOverlay state={state} />
      </Show>
    </View>
  );
}
