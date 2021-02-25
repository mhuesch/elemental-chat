let
  src = builtins.fetchGit {
    url = "git@github.com:holochain/holonix";
    rev = "044fa4b4e3db5a0e0274c0461628255c9c38aeb9";
  };
in

import src {}
