#!/usr/bin/env bash

# The cross-repository half of the gate.
#
#   ./build-and-test.sh          every sibling, built and tested as one
#
# Each repository gates itself — `bin/gate-common.sh` plus its own
# `build-and-test.sh`. This runs all nine of those, checks that the nine copies
# of the shared half are still one file, and then gates what no single
# repository can see: that they still work *together*, and that they agree
# about the versions of the third-party crates whose types cross between them.
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

# Nine copies of one file, and they are the same file.
#
# `bin/gate-common.sh` is carried by every repository because there is no
# submodule and nothing is published. A copy is a fork with a delay on it
# unless something compares them, and this is that something — the only reason
# copying is acceptable at all.
#
# Compared against this repository's *first* sibling rather than a checked-in
# reference: there is no canonical copy, and inventing one here would give the
# file a tenth home nobody edits.
gates_agree() {
  say "Every repository carries the same gate-common.sh"

  local sums missing="" repo
  for repo in "${SIBLINGS[@]}"; do
    if [ ! -f "../${repo}/bin/gate-common.sh" ]; then
      missing="${missing} ${repo}"
    fi
  done
  if [ -n "${missing}" ]; then
    echo "ERROR:${missing} carry no bin/gate-common.sh. Every repository gates" >&2
    echo "       itself; one that cannot is one whose links, warnings and" >&2
    echo "       oversized files nothing anywhere reads." >&2
    return 1
  fi

  # The monorepo too, while it still exists. It is not one of the nine — its
  # remote is the author's own and spec 52's step 2 has not been taken — but it
  # carries a tenth copy of this file, and a copy nobody compares is the whole
  # thing this check exists to prevent.
  local copies=("${SIBLINGS[@]/%//bin/gate-common.sh}")
  if [ -f "../xpui-framework/bin/gate-common.sh" ]; then
    copies+=("xpui-framework/bin/gate-common.sh")
  fi

  sums="$(cd .. && shasum -a 256 "${copies[@]}" | sort)"
  local distinct
  distinct="$(printf '%s\n' "${sums}" | awk '{print $1}' | sort -u | wc -l | tr -d ' ')"
  if [ "${distinct}" -ne 1 ]; then
    printf '%s\n' "${sums}" | sed 's/^/      /' >&2
    echo "ERROR: ${distinct} different versions of gate-common.sh above. It is" >&2
    echo "       one file with nine copies; edit one and copy it to the rest." >&2
    return 1
  fi
  printf '    one file, %s copies\n' "${#copies[@]}"
}

# Each repository's own gate, run from its own root.
#
# `xpui-dev/build-and-test.sh` used to open by saying "each repository gates
# itself" while none of them could. This is the line that makes it true.
every_repository_gates() {
  say "Every repository gates itself"
  local repo failures=""
  for repo in "${SIBLINGS[@]}"; do
    printf '\n--- %s\n' "${repo}"
    if ! (cd "../${repo}" && ./build-and-test.sh); then
      failures="${failures} ${repo}"
    fi
  done
  if [ -n "${failures}" ]; then
    echo "ERROR:${failures} failed their own gate." >&2
    return 1
  fi
}

# Every organisation URL names a path that is actually in that repository.
#
# This is the check whose absence cost sixty-one links twice over. `doc_paths`
# reads relative paths and says so deliberately; when the split turned those
# into `https://github.com/XPUI-Framework/…` URLs, it turned links a gate could
# read into links no gate anywhere could — and twenty-five of them named the
# monorepo's layout under the organisation's flat framework repository, so they
# were 404s from the moment they were written, in READMEs and in rustdoc.
#
# Only this repository can check them, because only this one has every sibling
# checked out. It resolves against `origin/main` rather than the working tree:
# a path that exists only locally is a link that is broken for everybody else,
# which is the entire failure mode.
#
# **`xpui-framework` is the flat framework crate, not the monorepo.** The
# monorepo lives on the author's own remote under the same name, and confusing
# the two is what produced the twenty-five.
org_links_resolve() {
  say "Every organisation URL names a file that is there"

  local urls broken=0 url repo path dir tree
  urls="$(cd .. && grep -rhoE \
    'https://github\.com/XPUI-Framework/[a-z0-9-]+/(blob|tree)/main/[^)# ]*' \
    --include='*.md' --include='*.rs' --include='*.toml' \
    --exclude-dir=target "${SIBLINGS[@]}" 2>/dev/null | sort -u || true)"

  # No URLs at all means the grep broke, not that the tree is clean.
  if [ -z "${urls}" ]; then
    echo "ERROR: no organisation URLs found anywhere, which cannot be right." >&2
    return 1
  fi

  while IFS= read -r url; do
    repo="${url#https://github.com/XPUI-Framework/}"
    repo="${repo%%/*}"
    path="${url#*/main/}"
    dir="../${repo}"
    # The organisation's `xpui-framework` is the crate, checked out as `xpui`.
    [ "${repo}" = "xpui-framework" ] && dir="../xpui"
    if [ ! -d "${dir}/.git" ]; then
      printf '  %s: no checkout of %s to check it against\n' "${url}" "${repo}" >&2
      broken=$((broken + 1))
      continue
    fi
    tree="$(git -C "${dir}" ls-tree -r --name-only origin/main)"
    if printf '%s\n' "${tree}" | grep -qx "${path}" \
       || printf '%s\n' "${tree}" | grep -q "^${path}/"; then
      continue
    fi
    printf '  %s\n' "${url}" >&2
    broken=$((broken + 1))
  done <<< "${urls}"

  if [ "${broken}" -gt 0 ]; then
    echo "ERROR: ${broken} organisation URL(s) above name nothing on that" >&2
    echo "       repository's main branch. A 404 in a README is worse than a" >&2
    echo "       relative path, because nothing but this line reads it." >&2
    return 1
  fi
  printf '    %s URLs, all resolved\n' "$(printf '%s\n' "${urls}" | wc -l | tr -d ' ')"
}

siblings_are_present
stays_local
gates_agree
org_links_resolve
locks_agree
every_repository_gates
the_whole_stack_builds

printf '\nThe stack holds together.\n'
