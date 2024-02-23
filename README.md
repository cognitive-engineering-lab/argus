# Argus: a Trait Debugger for Rust

[![tests](https://github.com/cognitive-engineering-lab/argus/actions/workflows/ci.yml/badge.svg)](https://github.com/cognitive-engineering-lab/argus/actions/workflows/ci.yml)

Argus is a tool to help you with compiler errors related to traits. If you have ever seen an error that says `the trait bound ... is not satisfied`, that is a good opportunity to use Argus. An IDE exension is available for VSCode which provides the Argus Inspection Panel.

## Limitations

> :warning: **Argus is research software and under active development!** :warning:

Argus relies on the New Trait Solver for Rust. Therefore, Argus inherets all the limitations of that solver which is also _under active development_. The New Trait Solver is known to be unsound and incomplete — while using Argus you may accidentally run into these areas. This does not mean that Argus is useless. The New Trait Solver is only used to type-check the current workspace and still works if you're using a trait-heavy crate.

## Installation

Argus is available as a VSCode extension. You can install Argus from the [VSCode Marketplace]() or the [Open VSX Registry](). In VSCode:

- Go to the extensions panel by clicking this button in the left margin:
- Search for "Argus" and click "Install".
- Open a Rust workspace and wait for Argus to finish installing.

### Building from source

Some additional software is needed to build Argus from source. For the TypeScript bindings you need to install the language [Guile](https://www.gnu.org/software/guile/). The frontend requires [Depot](https://github.com/cognitive-engineering-lab/depot), a JS "devtool orchestrator." After this simply run the following:

```sh
$ cargo make init-bindings

$ cargo install --path crates/argus_cli

$ cd ide && depot build
```

## FAQ

<h3 id="rustup-fails-on-install">rustup fails on installation</h3>

If rustup fails, especially with an error like "could not rename downloaded file", this is probably because Argus is running rustup concurrently with another tool (like rust-analyzer). Until [rustup#988](https://github.com/rust-lang/rustup/issues/988) is resolved, there is unfortunately no automated way around this.

To solve the issue, go to the command line and run:

```
rustup toolchain install nightly-2024-01-24 -c rust-src -c rustc-dev -c llvm-tools-preview
```

Then go back to VSCode and click "Continue" to let Argus continue installing.

### Where does the name come from?

Argus or Argos Panoptes (Ancient Greek: Ἄργος Πανόπτης, "All-seeing Argos") is a many-eyed giant in Greek mythology.

> Argus was Hera's servant. His great service to the Olympian pantheon was to slay the chthonic serpent-legged monster Echidna as she slept in her cave. Hera's defining task for Argus was to guard the white heifer Io from Zeus, who was attracted to her, keeping her chained to the sacred olive tree at the Argive Heraion. She required someone who had at least a hundred eyes spread out, always watching in all directions, someone who would stay awake despite being asleep. _Argos was meant to be the perfect guardian_.

—[Wikipedia](https://en.wikipedia.org/wiki/Argus_Panoptes)
