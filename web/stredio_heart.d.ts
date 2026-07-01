/* tslint:disable */
/* eslint-disable */

export class AddonRuntime {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * Current add-on list (array of descriptors) as JSON — what the shell renders.
     */
    addons_json(): string;
    /**
     * Current install map (id -> installed) as JSON.
     */
    install_map_json(): string;
    install_state_pulled(map_json: string, at: number, owner_changed: boolean): string;
    load_official(): string;
    /**
     * `inline_json` is the inline default add-on array (the boot/fallback set).
     */
    constructor(inline_json: string);
    official_manifest_fetched(json?: string | null): string;
    official_payload_fetched(json?: string | null): string;
    /**
     * Load status (`"Idle" | "Loading" | "Loaded" | "Failed"`).
     */
    status(): string;
    toggle_addon(id: string, now: number): string;
}

export class CatalogRuntime {
    free(): void;
    [Symbol.dispose](): void;
    hero_fetched(item_json?: string | null): string;
    hydrate_row_config(cfg_json: string): string;
    load_home(): string;
    constructor();
    row_fetched(cat: string, items_json?: string | null): string;
    set_gating(catalog: boolean, providers: boolean, studios: boolean): string;
    /**
     * The whole home-rows model as JSON (gating, config, loaded rows, hero).
     */
    snapshot_json(): string;
    toggle_row(cat: string, on: boolean): string;
    /**
     * The ordered list of visible row categories, as a JSON array of strings.
     */
    visible_rows_json(): string;
}

export class LibraryRuntime {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The Continue-Watching items (unfinished), newest first, as JSON.
     */
    continue_watching_json(): string;
    /**
     * Load a persisted `Library` (`{history, progress, removed}`) at boot.
     */
    hydrate(library_json: string): string;
    constructor();
    /**
     * Merge server state (`GET /api/library-state`) by recency.
     */
    pulled(history_json: string, progress_json: string, removed_json: string, now: number): string;
    /**
     * Record a watched/opened title (`item_json` is a `LibraryItem`).
     */
    record_watch(item_json: string): string;
    remove(id: string, now: number): string;
    set_progress(id: string, pos: number, dur: number, now: number): string;
    /**
     * The whole library (`{history, progress, removed}`) as JSON — for local
     * persistence and the server push body.
     */
    snapshot_json(): string;
}

export function collection_addons_json(payload_json: string): string;

export function merge_official_json(inline_json: string, cdn_json: string): string;

export function official_payload_file(manifest_json: string): string;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_addonruntime_free: (a: number, b: number) => void;
    readonly __wbg_catalogruntime_free: (a: number, b: number) => void;
    readonly __wbg_libraryruntime_free: (a: number, b: number) => void;
    readonly addonruntime_addons_json: (a: number, b: number) => void;
    readonly addonruntime_install_map_json: (a: number, b: number) => void;
    readonly addonruntime_install_state_pulled: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly addonruntime_load_official: (a: number, b: number) => void;
    readonly addonruntime_new: (a: number, b: number) => number;
    readonly addonruntime_official_manifest_fetched: (a: number, b: number, c: number, d: number) => void;
    readonly addonruntime_official_payload_fetched: (a: number, b: number, c: number, d: number) => void;
    readonly addonruntime_status: (a: number, b: number) => void;
    readonly addonruntime_toggle_addon: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly catalogruntime_hero_fetched: (a: number, b: number, c: number, d: number) => void;
    readonly catalogruntime_hydrate_row_config: (a: number, b: number, c: number, d: number) => void;
    readonly catalogruntime_load_home: (a: number, b: number) => void;
    readonly catalogruntime_new: () => number;
    readonly catalogruntime_row_fetched: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly catalogruntime_set_gating: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly catalogruntime_snapshot_json: (a: number, b: number) => void;
    readonly catalogruntime_toggle_row: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly catalogruntime_visible_rows_json: (a: number, b: number) => void;
    readonly collection_addons_json: (a: number, b: number, c: number) => void;
    readonly libraryruntime_continue_watching_json: (a: number, b: number) => void;
    readonly libraryruntime_hydrate: (a: number, b: number, c: number, d: number) => void;
    readonly libraryruntime_new: () => number;
    readonly libraryruntime_pulled: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => void;
    readonly libraryruntime_record_watch: (a: number, b: number, c: number, d: number) => void;
    readonly libraryruntime_remove: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly libraryruntime_set_progress: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => void;
    readonly libraryruntime_snapshot_json: (a: number, b: number) => void;
    readonly merge_official_json: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly official_payload_file: (a: number, b: number, c: number) => void;
    readonly __wbindgen_add_to_stack_pointer: (a: number) => number;
    readonly __wbindgen_export: (a: number, b: number, c: number) => void;
    readonly __wbindgen_export2: (a: number, b: number) => number;
    readonly __wbindgen_export3: (a: number, b: number, c: number, d: number) => number;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
