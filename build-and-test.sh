#!/usr/bin/env bash

# The cross-repository half of the gate.
#
#   ./build-and-test.sh          every sibling, built and tested as one
#
# Each repository gates itself. This gates what no single repository can see:
# that the nine still work *together*, and that they agree about the versions
# of the third-party crates whose types cross between them.
#
# Run it before pushing a change that touches more than one repository. After
# the push, every repository resolves its siblings from GitHub at whatever
# revision its own lock file names, and a cross-cutting change reaches them one
# `cargo update` at a time — this is the only place the whole stack is one
# build.

set -euo pipefail

DEV_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "${DEV_DIR}"

# Every repository, as a directory beside this one. `xpui` is the framework
# crate; its repository is called `xpui-framework`.
SIBLINGS=(xpui xpui-chrome xpui-boards xpui-backends xpui-simulator xpui-gallery
          xpui-rp2040 xpui-esp32 xpui-cpp)

# Third-party crates whose **types** cross a repository boundary. A skew in one
# of these is not a warning: `DrawTarget` from 0.8.1 is a different type from
# `DrawTarget` from 0.8.2, and the error names the same path twice.
SHARED_CRATES=(embedded-graphics embedded-graphics-core critical-section u8g2-fonts)

say() { printf '\n==> %s\n' "$*"; }

siblings_are_present() {
  say "Every repository is checked out beside this one"
  local missing=0
  local repo
  for repo in "${SIBLINGS[@]}"; do
    if [ ! -d "../${repo}/.git" ]; then
      printf '  ../%s is not a git checkout\n' "${repo}" >&2
      missing=$((missing + 1))
    fi
  done
  if [ "${missing}" -gt 0 ]; then
    echo "ERROR: ${missing} missing. Clone them from github.com/XPUI-Framework." >&2
    return 1
  fi
}

# Nothing here may be pushed: the `[patch]` paths are this machine's, and a
# lock file resolved against them is meaningless anywhere else.
stays_local() {
  say "This repository has no remote"
  if git remote | grep -q .; then
    echo "ERROR: xpui-dev has a remote. Its [patch] paths are local to this" >&2
    echo "       machine; a lock file resolved against them means nothing" >&2
    echo "       elsewhere. Remove it." >&2
    return 1
  fi
}

the_whole_stack_builds() {
  say "Every crate, from local paths"
  cargo build --workspace
  cargo test --workspace
}

# Every lock file in the organisation agrees about the crates whose types cross
# between repositories.
#
# This existed inside the monorepo for two lock files and three crates, because
# `examples/rp2040` was already a separate workspace and the cost showed up
# immediately. There are more of both now, and the failure is worse: two
# repositories resolving different `embedded-graphics` patch releases produce
# types that do not satisfy each other, and the error blames a trait.
locks_agree() {
  say "Every lock file agrees about the shared crates"

  local disagreements=0
  local crate
  for crate in "${SHARED_CRATES[@]}"; do
    local seen=""
    local lock
    for lock in Cargo.lock ../*/Cargo.lock; do
      [ -f "${lock}" ] || continue
      local version
      version="$(awk -v c="${crate}" '
        $0 == "name = \"" c "\"" { want = 1; next }
        want && /^version = / { gsub(/[",]/, "", $3); print $3; exit }
      ' "${lock}")"
      [ -n "${version}" ] || continue
      seen="${seen}${version} ${lock}"$'\n'
    done

    local distinct
    distinct="$(printf '%s' "${seen}" | awk 'NF {print $1}' | sort -u | wc -l | tr -d ' ')"
    if [ "${distinct}" -gt 1 ]; then
      printf '  %s is resolved %s different ways:\n' "${crate}" "${distinct}" >&2
      printf '%s' "${seen}" | sed 's/^/      /' >&2
      disagreements=$((disagreements + 1))
    fi
  done

  if [ "${disagreements}" -gt 0 ]; then
    echo "ERROR: ${disagreements} crate(s) above. Their types cross repository" >&2
    echo "       boundaries, so a skew is a type mismatch rather than a" >&2
    echo "       warning. \`cargo update -p <crate>\` in the ones behind." >&2
    return 1
  fi
}

siblings_are_present
stays_local
locks_agree
the_whole_stack_builds

printf '\nThe stack holds together.\n'
