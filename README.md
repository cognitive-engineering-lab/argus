# <img src="https://github.com/cognitive-engineering-lab/argus/blob/main/ide/packages/extension/argus-logo-128.png?raw=true" height="64" /> Argus: a Trait Debugger for Rust

[![tests](https://github.com/cognitive-engineering-lab/argus/actions/workflows/ci.yml/badge.svg)](https://github.com/cognitive-engineering-lab/argus/actions/workflows/ci.yml)

Argus is a tool to help you with compiler errors related to traits. If you have ever seen an error that says `the trait bound ... is not satisfied`, that is a good opportunity to use Argus. An IDE extension is available for VSCode which provides the Argus Inspection Panel. Read the [trait debugging tutorial](https://cel.cs.brown.edu/argus/) to learn more.

## Limitations

> :warning: **Argus is research software and is under active development!** :warning:

Argus relies on the New Trait Solver for Rust. Therefore, Argus inherits all the limitations of that solver which is also _under active development_. The New Trait Solver is known to be incomplete — while using Argus you may accidentally run into areas of Rust where the solver is limited. This does not mean that Argus is useless. The New Trait Solver is only used to type-check the current workspace and is still useful to debug a wide-range of trait errors.


https://github.com/user-attachments/assets/9a544060-e8ad-455e-9831-5e5d48c96542


## Installation

Argus is available as a VSCode extension. You can install Argus from the [VSCode Marketplace](https://marketplace.visualstudio.com/items?itemName=gavinleroy.argus) or the [Open VSX Registry](https://open-vsx.org/extension/gavinleroy/argus). In VSCode:

- Go to the extensions panel by clicking this button in the left margin: <img width="53" alt="Screenshot 2024-02-23 at 23 26 58" src="https://github.com/cognitive-engineering-lab/argus/assets/20209337/10d9297e-3c2a-4866-854f-de79f7de11de">

- Search for "Argus" and click "Install".
- Open a Rust workspace and wait for Argus to finish installing.

### Building from source

Some additional software is needed to build Argus from source. For the TypeScript bindings, you need to install the language [Guile](https://www.gnu.org/software/guile/). The IDE requires [Depot](https://github.com/cognitive-engineering-lab/depot). Afterward run

```sh
$ cargo make init-bindings

$ cargo install --path crates/argus-cli

$ cd ide && depot build
```

## FAQ

<h3 id="rustup-fails-on-install">rustup fails on installation</h3>

If rustup fails, especially with an error like "could not rename the downloaded file", this is probably because Argus is running rustup concurrently with another tool (like rust-analyzer). Until [rustup#988](https://github.com/rust-lang/rustup/issues/988) is resolved, there is, unfortunately, no automated way around this.

To solve the issue, go to the command line and run:

```
rustup toolchain install nightly-2024-12-15 -c rust-src -c rustc-dev -c llvm-tools-preview
```

Then go back to VSCode and click "Continue" to let Argus continue installing.

### Where does the name come from?

Argus or Argos Panoptes (Ancient Greek: Ἄργος Πανόπτης, "All-seeing Argos") is a many-eyed giant in Greek mythology.

> Argus was Hera's servant. His great service to the Olympian pantheon was to slay the chthonic serpent-legged monster Echidna as she slept in her cave. Hera's defining task for Argus was to guard the white heifer Io from Zeus, who was attracted to her, keeping her chained to the sacred olive tree at the Argive Heraion. She required someone who had at least a hundred eyes spread out, always watching in all directions, someone who would stay awake despite being asleep. _Argos was meant to be the perfect guardian_.

—[Wikipedia](https://en.wikipedia.org/wiki/Argus_Panoptes)

## Having trouble? (or providing feedback!)

We're happy to chat and answer questions on the [Argus Zulip channel](https://cognitive-engineering-lab.zulipchat.com/#narrow/stream/453634-argus). You can also reach the authors via email, <gavinleroy@brown.edu> and <wcrichto@brown.edu>, we'd love to hear your feedback as we iterate in the design and development of Argus!
