default:
    just --list

[group("dev")]
@podman_run:
    podman compose up --build

[group("dev")]
podman_down:
    podman compose down
