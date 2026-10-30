# Dioxus

The template is written against **Dioxus 0.8.0-alpha.3** from the
[g3 fork](https://github.com/Gateway3B/dioxus) of Dioxus `main` (`dioxus`,
`dioxus-fullstack` and `manganis` pinned with `=` and patched to the fork, to
match a `dx` built from it). The [README](../../README.md#dioxus-08-and-the-g3-fork)
says why.

| File | What it is |
| --- | --- |
| [patterns.md](patterns.md) | **Start here.** How the g3 apps use Dioxus: props, hooks, stores, cached data, hydration, and the tooling pitfalls we have hit. Ours; kept current. |
| [framework-agents.md](framework-agents.md) | Dioxus's own agent guide, from the framework repository. |
| [architecture/](architecture/00-OVERVIEW.md) | The framework's internal architecture notes: core, CLI, RSX, signals and stores, fullstack, renderers, hot reload, assets, router, WASM splitting, native plugins, manifests. |

`framework-agents.md` and `architecture/` are copied unchanged from the fork's
`main` (upstream [DioxusLabs/dioxus](https://github.com/DioxusLabs/dioxus) `main`
plus the g3 commits, a few of which add notes). They describe the framework,
not this template; each file's first line says so and names the commit.
Refresh them from the fork when it moves, and from upstream once 0.8.0 is
published; do not edit them here.

Where they and `patterns.md` disagree about how to write code *in this app*,
`patterns.md` wins.
