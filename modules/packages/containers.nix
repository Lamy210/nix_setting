{ pkgs, ... }:
{
  home.packages = with pkgs; [
    colima
    docker
    docker-compose
    kubectl
    kubernetes-helm-wrapped
    kind
    lazydocker
    terraform
  ];
}
