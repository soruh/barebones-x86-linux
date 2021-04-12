#!/bin/sh

exec pandoc slides.md -o slides.pdf -f gfm+tex_math_dollars -t beamer --highlight-style zenburn --filter pandoc-latex-fontsize $@
# espresso, zenburn, tango
