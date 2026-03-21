set -euo pipefail

gh_tag="${DLL_PACK_GH_TAG:-${GITHUB_REF_NAME:-${GITHUB_REF:-}}}"
gh_tag="${gh_tag#refs/tags/}"
if [ -z "${gh_tag}" ]; then
    gh_tag="${GITHUB_SHA:-local}"
fi

sysroot="$(rustc --print sysroot | sed 's|\\|/|g')"
host_lib_dir="${sysroot}/lib/rustlib/$(rustc -vV | grep host | awk '{print $2}')/lib"
win_path="${host_lib_dir}"

extra_dll_pack_args=(
    --include "${host_lib_dir}/*"
    --macho-rpath "${host_lib_dir}"
)

if [ -n "${SYSTEMROOT:-}" ]; then
    system_root="$(printf '%s\n' "${SYSTEMROOT}" | sed 's|\\|/|g')"
    win_path="${host_lib_dir};${system_root}/System32;${system_root}/SysWOW64"
fi

extra_dll_pack_args+=(
    --win-path "${win_path}"
)

rustup target add "${DLL_PACK_TARGET}"

cargo build --profile super-release --target "${DLL_PACK_TARGET}"

target_dll="$(dll-pack-builder find "${BUILD_OUT_DIR}")"

mkdir -p ./artifacts/

LD_LIBRARY_PATH="${host_lib_dir}" \
    dll-pack-builder local $(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].name') \
    "${target_dll}" \
    ./artifacts/ "${DLL_PACK_TARGET}" "${GITHUB_REPOSITORY}" "${gh_tag}" \
    "${extra_dll_pack_args[@]}"
