// Pocket Shelf: lists the titles installed on the device and launches them.
//
//   state.ts        selection, theme, view mode, menu, icon loading
//   input.ts        button bindings
//   themes.ts       color palettes
//   text.ts         fonts and the text class per role
//   catalog.ts      reads the host's installed-title table
//   navigation.ts   view modes and D-pad movement per view
//   views/          shelf (carousel), grid, list and cascade
//   components/     title art, header, footer, drawers, keyboard

import { createSignal, onMount, Show } from "solid-js";
import { Text, View } from "@pocketjs/framework/components";
import { fitTitle, loadCatalog, titlesReady } from "./catalog.ts";
import { TINT_AMBIENT } from "./components/art.tsx";
import { AdrenalineOverlay } from "./components/adrenaline.tsx";
import { ArtPickerOverlay } from "./components/art-picker.tsx";
import { Backdrop } from "./components/backdrop.tsx";
import { CategoryManagerOverlay } from "./components/category-manager.tsx";
import { CleanupOverlay } from "./components/cleanup.tsx";
import { DiagnosticsOverlay } from "./components/diagnostics.tsx";
import { EditorOverlay } from "./components/editor.tsx";
import { Footer } from "./components/footer.tsx";
import { Header } from "./components/header.tsx";
import { StatusBar } from "./components/status-bar.tsx";
import { hints, Prompt, withButton } from "./components/icons.tsx";
import { KeyboardOverlay } from "./components/keyboard.tsx";
import { MenuOverlay } from "./components/menu.tsx";
import { OnlineOverlay } from "./components/online.tsx";
import { RescanOverlay } from "./components/rescan.tsx";
import { ScanPage } from "./components/scan-page.tsx";
import { ScrapeOverlay } from "./components/scrape.tsx";
import { UpdateOverlay } from "./components/update.tsx";
import { setupDone, timed } from "./diagnostics.ts";
import { fill } from "./i18n.ts";
import { installInput } from "./input.ts";
import { createLauncherState } from "./state.ts";
import { fontSlot } from "./text.ts";
import { CarouselView } from "./views/carousel.tsx";
import { CascadeView } from "./views/cascade.tsx";
import { GridView } from "./views/grid.tsx";
import { ListView } from "./views/list.tsx";

/**
 * The shelf, after the scan page on a start with no title list yet (the
 * first one, or after the list file was deleted).
 */
export default function Root() {
  const [ready, setReady] = createSignal(titlesReady());
  return (
    <Show when={ready()} fallback={<ScanPage onDone={() => setReady(true)} />}>
      <Shelf />
    </Show>
  );
}

function Shelf() {
  setupDone();
  const catalog = timed("catalog", loadCatalog);
  const state = timed("state", () => createLauncherState(catalog));

  onMount(() => installInput(state));

  const selected = () => state.games()[state.selectedIndex()];

  return timed("tree", () => (
    <View
      class="relative flex-col w-full h-full bg-gradient-to-b from-black to-black"
      style={{ gradFrom: state.theme().bgTop, gradTo: state.theme().bgBottom }}
    >
      {/* The selected title's picture, or its tint washed over the top of the screen. */}
      {/* The Dynamic theme already colors the screen from the title, so it has no wash. */}
      <Show when={!state.backdrop() && state.theme().id !== "dynamic" ? selected() : undefined}>
        {(game) => <View class={TINT_AMBIENT[game().tint % TINT_AMBIENT.length]} />}
      </Show>
      {timed("backdrop", () => (
        <Backdrop state={state} />
      ))}

      <Show when={state.statusBarOn()}>
        <StatusBar state={state} />
      </Show>
      {timed("header", () => (
        <Header state={state} />
      ))}

      {timed("view", () => (
      <Show
        when={state.games().length > 0}
        fallback={
          <View class="flex-col w-full grow items-center justify-center gap-1">
            <Show
              when={state.searching()}
              fallback={
                <>
                  <Text class={state.text().title} style={{ textColor: state.theme().text }}>
                    {state.hasTitles() ? state.t().emptyCategory : state.t().noTitles}
                  </Text>
                  <Show when={state.hasTitles()}>
                    <Prompt state={state} parts={withButton(state.t().moveHere, "triangle")} />
                  </Show>
                </>
              }
            >
              <Text class={state.text().title} style={{ textColor: state.theme().text }}>
                {fill(state.t().noMatch, {
                  term: fitTitle(state.searchTerm(), 180, fontSlot(state.font(), "title")),
                })}
              </Text>
              <Prompt
                state={state}
                parts={hints(["square", state.t().searchAgain], [state.cancelButton(), state.t().close])}
              />
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
          <Show when={state.view() === "cascade"}>
            <CascadeView state={state} />
          </Show>
        </Show>
      </Show>
      ))}

      {timed("footer", () => (
        <Footer state={state} />
      ))}

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
      <Show when={state.scrape.open()}>
        <ScrapeOverlay state={state} />
      </Show>
      <Show when={state.modal() === "clean"}>
        <CleanupOverlay state={state} />
      </Show>
      <Show when={state.modal() === "rescan"}>
        <RescanOverlay state={state} />
      </Show>
      <Show when={state.update.open()}>
        <UpdateOverlay state={state} />
      </Show>
      <Show when={state.modal() === "diagnostics"}>
        <DiagnosticsOverlay state={state} />
      </Show>
      <Show when={state.modal() === "art"}>
        <ArtPickerOverlay state={state} />
      </Show>
      <Show when={state.modal() === "keyboard"}>
        <KeyboardOverlay state={state} />
      </Show>
      <Show when={state.modal() === "adrenaline"}>
        <AdrenalineOverlay state={state} />
      </Show>
    </View>
  ));
}
