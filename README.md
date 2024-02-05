# Argus

Argus or Argos Panoptes (Ancient Greek: Ἄργος Πανόπτης, "All-seeing Argos") is a many-eyed giant in Greek mythology.

> Argus was Hera's servant. His great service to the Olympian pantheon was to slay the chthonic serpent-legged monster Echidna as she slept in her cave. Hera's defining task for Argus was to guard the white heifer Io from Zeus, who was attracted to her, keeping her chained to the sacred olive tree at the Argive Heraion. She required someone who had at least a hundred eyes spread out, always watching in all directions, someone who would stay awake despite being asleep. *Argos was meant to be the perfect guardian*.

—([Wikipedia](https://en.wikipedia.org/wiki/Argus_Panoptes))

## Building locally

Some additional software is needed to build Argus from scratch.
For the TypeScript bindings you need to install the language [Guile](https://www.gnu.org/software/guile/). The frontend requires [Depot](https://github.com/cognitive-engineering-lab/depot), a JS "devtool orchestrator." After this simply run the following:

``` sh
$ cargo make init-bindings

$ cargo install --path crates/argus_cli

$ cd ide && depot build
```
