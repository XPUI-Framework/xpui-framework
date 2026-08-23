#!/usr/bin/env bash

# Everything CI checks in this repository, in one command.
#
#   ./build-and-test.sh          format, lint, test, and every documented snippet
#   ./build-and-test.sh check    the same thing; the name CI uses
#   ./build-and-test.sh fix      format Rust in place first
#
# **Half of what runs is in `bin/gate-common.sh`**, of which every repository
# in the organisation carries a byte-identical copy. This file is what this
# repository configures, what only it checks, and the order they run in.
# `xpui-dev` compares the nine copies and runs all nine gates.
#
# The framework, alone. It has no dependencies and no product in it, which is
# what `framework_is_generic` below is here to keep true.

set -euo pipefail

PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${PROJECT_DIR}"

# One crate, at the root. `file_sizes` prunes `target`, so an extracted
# package's `src/` is not read as ours.
SOURCE_ROOTS=(.)

# `testing` installs the fake host, which the unit tests and several of the
# guides' snippets need.
TEST_FEATURES="testing"

# The framework runs wherever a backend does, so it is linted for both
# bare-metal architectures. Neither has atomic compare-and-swap; the second is
# not the stricter run, it is the second architecture. The `?` says Cortex-M0+
# may skip when the target is not installed rather than failing a gate somebody
# cannot fix without a download.
HOST_WORKSPACE=1
LINT_TARGETS=("riscv32imc-unknown-none-elf" "thumbv6m-none-eabi?")
LINT_TARGET_CRATES=(-p xpui)

# The framework may not name a product, a device or a backend. Eleven words,
# greped over its source.
#
# **Scoped to `src/`, and it stays that way.** Grepping the repository root
# would read this file, the README and every guide — and a guide that explains
# what a backend is has to be able to say the word.
GENERIC_ROOT="src/"
GENERIC_FORBIDDEN="crosspoint|xteink|freeink|embedded_graphics|badger|tufty|pimoroni|seeed|sticky|x4 ?pro|gfxrenderer"

. bin/gate-common.sh

# ---------------------------------------------------------------------------
# What only this repository checks.
# ---------------------------------------------------------------------------

# No product, no device, no backend — by name, anywhere in the framework.
#
# A review comment does not run. This does, and it is the reason the framework
# can be depended on by a backend nobody here has written: the moment `xpui`
# knows what a Badger is, a second device is a special case rather than a
# `Board`.
framework_is_generic() {
  say "The framework names no product and no backend"

  if [ ! -d "${GENERIC_ROOT}" ]; then
    echo "ERROR: GENERIC_ROOT is ${GENERIC_ROOT}, which is not a directory." >&2
    echo "       A grep over a missing root finds nothing, which reads exactly" >&2
    echo "       like a framework that names nothing." >&2
    return 1
  fi

  if grep -rniE "${GENERIC_FORBIDDEN}" "${GENERIC_ROOT}" >&2; then
    echo "ERROR: the framework names something above that it may not know about." >&2
    echo "       A device belongs in a boards crate; a backend behind a trait." >&2
    echo "       If the framework needs something from one, add a trait method." >&2
    return 1
  fi
  echo "    clean"
}

# ---------------------------------------------------------------------------

gates() {
  framework_is_generic
  file_sizes
  every_check_runs
  readmes_warn
  prose_is_compiled
  doc_paths
  commands_resolve
  cpp_snippets_compile
  lint
  test_suite
  doc_tests
  doc_links
}

case "${1:-check}" in
  check)
    run_all "${FORMAT_CHECK[@]}"
    gates
    printf '\nChecks passed.\n'
    ;;
  fix)
    run_all "${FORMAT_FIX[@]}"
    gates
    printf '\nFormatted and checked.\n'
    ;;
  *)
    echo "usage: ./build-and-test.sh [check|fix]" >&2
    exit 2
    ;;
esac
