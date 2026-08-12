{ pkgs, ... }:
{
  home.packages = with pkgs; [
    pgcli
    mycli
    usql
    redis
    sqlite
    protobuf
    grpcurl
    websocat
    bruno
  ];
}
