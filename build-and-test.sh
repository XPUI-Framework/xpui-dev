#!/usr/bin/env bash

# The cross-repository half of the gate.
#
#   ./build-and-test.sh          every sibling, built and tested as one
#   ./build-and-test.sh cross    only what no single repository can check
#
# The second exists for CI. Every repository runs its own gate in its own
# workflow, so running all nine again here would pay twice for the same
# checks — `cross` skips `every_repository_gates` and keeps the four that are
# genuinely cross-repository. On a laptop, before a push, use the first: there
# the siblings hold uncommitted changes their own CI has never seen.
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

# The files every repository carries a copy of, and which must not drift.
#
# There is no submodule and nothing is published, so a handful of files are
# **copied** into every repository: the shared half of the gate, the licence,
# and clippy's configuration. A copy nobody compares is a fork with a delay on
# it — and that is not hypothetical here. The nine repositories ran clippy
# under *different settings* from the monorepo for as long as `clippy.toml`
# existed in only one of them, and nothing said so, because the check that
# existed compared `gate-common.sh` and only `gate-common.sh`.
#
# `rust-toolchain.toml` is the one exception, and it is checked differently:
# its `targets` list legitimately differs per repository, so only the `channel`
# line has to agree. Repositories bumping to different compilers is exactly the
# drift this prevents.
# `bin/gate-common.sh` is the nine and the monorepo; this repository has no
# copy, because it *is* the cross-repository half and shares no checks with
# them. Everything else is carried here too and is compared here too — a
# licence or a lint setting that drifts in the umbrella is still drift.
SHARED_FILES=("bin/gate-common.sh" "LICENSE" "clippy.toml")
SHARED_FILES_HERE_TOO=("LICENSE" "clippy.toml")

shared_files_agree() {
  say "Every repository carries the same copy of each shared file"

  # The monorepo too, while it exists. It is not one of the nine — its remote
  # is the author's own — but it carries the same copies, and a copy nobody
  # compares is the whole thing this check exists to prevent.
  local roots=("${SIBLINGS[@]}")
  if [ -d "../xpui-framework/.git" ]; then
    roots+=("xpui-framework")
  fi

  local failures=0 file root missing sums distinct here
  for file in "${SHARED_FILES[@]}"; do
    # This repository's own copy, for the files it carries.
    here=("${roots[@]}")
    case " ${SHARED_FILES_HERE_TOO[*]} " in
      *" ${file} "*) here+=("xpui-dev") ;;
    esac
    missing=""
    for root in "${here[@]}"; do
      [ -f "../${root}/${file}" ] || missing="${missing} ${root}"
    done
    if [ -n "${missing}" ]; then
      printf '  %s is absent from:%s\n' "${file}" "${missing}" >&2
      failures=$((failures + 1))
      continue
    fi

    sums="$(cd .. && shasum -a 256 "${here[@]/%//${file}}" | sort)"
    distinct="$(printf '%s\n' "${sums}" | awk '{print $1}' | sort -u | wc -l | tr -d " ")"
    if [ "${distinct}" -ne 1 ]; then
      printf '%s\n' "${sums}" | sed "s/^/      /" >&2
      printf '  %s has %s different versions above\n' "${file}" "${distinct}" >&2
      failures=$((failures + 1))
      continue
    fi
    printf '    %-22s one file, %s copies\n' "${file}" "${#here[@]}"
  done

  # The toolchain: same compiler everywhere, whatever targets each installs.
  local channels
  channels="$(cd .. && for root in "${roots[@]}" xpui-dev; do
    grep -h "^channel" "${root}/rust-toolchain.toml" 2>/dev/null || echo "MISSING ${root}"
  done | sort -u)"
  if [ "$(printf '%s\n' "${channels}" | wc -l | tr -d " ")" -ne 1 ]; then
    printf '%s\n' "${channels}" | sed "s/^/      /" >&2
    echo "  rust-toolchain.toml names more than one channel above" >&2
    failures=$((failures + 1))
  else
    printf "    %-22s %s\n" "rust-toolchain.toml" "${channels}"
  fi

  if [ "${failures}" -gt 0 ]; then
    echo "ERROR: ${failures} shared file(s) above have drifted. Each is one file" >&2
    echo "       with a copy per repository; edit one and copy it to the rest." >&2
    return 1
  fi
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

# The FreeInk SDK revision, written down in two repositories.
#
# `xpui-backends` compiles the shim against the SDK's headers and `xpui-cpp`
# links it, so each pins a revision — and a revision written down twice is a
# revision that will disagree with itself. The monorepo kept one file that
# CMake and CI both read; the split gave them one each, and this is what stops
# them drifting.
sdk_revisions_agree() {
  say "Both repositories pin the same FreeInk SDK revision"

  local backends="../xpui-backends/fui/freeink-sdk.rev"
  local cpp="../xpui-cpp/cpp_host/freeink-sdk.rev"
  local missing=0 file
  for file in "${backends}" "${cpp}"; do
    if [ ! -f "${file}" ]; then
      printf '  %s is not there\n' "${file}" >&2
      missing=$((missing + 1))
    fi
  done
  if [ "${missing}" -gt 0 ]; then
    echo "ERROR: a repository that compiles against the SDK pins no revision," >&2
    echo "       so its CI would clone whatever main happens to be." >&2
    return 1
  fi

  if ! diff -q "${backends}" "${cpp}" >/dev/null; then
    printf '  xpui-backends: %s\n' "$(cat "${backends}")" >&2
    printf '  xpui-cpp:      %s\n' "$(cat "${cpp}")" >&2
    echo "ERROR: the two pins differ. The shim is syntax-checked against one" >&2
    echo "       SDK and linked against another, and the error surfaces at the" >&2
    echo "       link with no hint that a revision is the cause." >&2
    return 1
  fi
  printf '    %s\n' "$(cat "${backends}")"
}

cross_repository_only() {
  siblings_are_present
  shared_files_agree
  sdk_revisions_agree
  org_links_resolve
  locks_agree
  the_whole_stack_builds
}

case "${1:-all}" in
  all)
    cross_repository_only
    every_repository_gates
    printf '\nThe stack holds together.\n'
    ;;
  cross)
    cross_repository_only
    printf '\nThe cross-repository checks pass. Each gate runs in its own CI.\n'
    ;;
  *)
    echo "usage: ./build-and-test.sh [all|cross]" >&2
    exit 2
    ;;
esac
