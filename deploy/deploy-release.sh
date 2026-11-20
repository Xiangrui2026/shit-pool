#!/usr/bin/env bash
set -euo pipefail
umask 077

deployment=/opt/shit-pool
registry=ghcr.io/xiangrui2026/shit-pool
pointer="$registry-api:production"
export COMPOSE_PARALLEL_LIMIT=1

exec 9>"$deployment/.deploy.lock"
flock -n 9 || exit 0

docker pull --quiet "$pointer" >/dev/null
read -r revision compose_sha <<< "$(docker image inspect "$pointer" --format \
    '{{index .Config.Labels "org.opencontainers.image.revision"}} {{index .Config.Labels "io.shitpool.compose-sha256"}}')"
if [[ ! $revision =~ ^[0-9a-f]{40}$ || ! $compose_sha =~ ^[0-9a-f]{64}$ ]]; then
    echo 'Production image has invalid release metadata.' >&2
    exit 1
fi

previous=
if [[ -f $deployment/current ]]; then
    read -r previous < "$deployment/current"
fi
if [[ $revision == "$previous" ]]; then
    exit 0
fi

echo "Deploying $revision"
compose_file="$deployment/releases/$revision.yaml"
staged=$(mktemp "$deployment/releases/.compose.XXXXXX")
trap 'rm -f -- "$staged"' EXIT
curl --fail --silent --show-error --location --retry 3 --max-time 30 \
    "https://raw.githubusercontent.com/Xiangrui2026/shit-pool/$revision/compose.prod.yaml" -o "$staged"
if ! printf '%s  %s\n' "$compose_sha" "$staged" | sha256sum --check --status; then
    echo 'Compose file does not match the published release.' >&2
    exit 1
fi
mv -fT -- "$staged" "$compose_file"

compose() {
    IMAGE_TAG="$revision" docker compose --project-name shit-pool --project-directory "$deployment" \
        --env-file "$deployment/.env.production" -f "$compose_file" "$@"
}
compose config --quiet
compose pull --quiet
compose up --detach --wait --wait-timeout 120

staged=$(mktemp "$deployment/.current.XXXXXX")
printf '%s\n' "$revision" > "$staged"
mv -fT -- "$staged" "$deployment/current"
echo "Deployed $revision"

if [[ $previous =~ ^[0-9a-f]{40}$ ]]; then
    for component in web api worker ccho; do
        docker image rm "$registry-$component:$previous" >/dev/null || true
    done
fi
