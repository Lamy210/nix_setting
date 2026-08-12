_: {
  system = {
    defaults = {
      NSGlobalDomain = {
        AppleKeyboardUIMode = 3;
        ApplePressAndHoldEnabled = false;
        InitialKeyRepeat = 15;
        KeyRepeat = 2;
        NSAutomaticCapitalizationEnabled = false;
        NSAutomaticDashSubstitutionEnabled = false;
        NSAutomaticPeriodSubstitutionEnabled = false;
        NSAutomaticQuoteSubstitutionEnabled = false;
        NSAutomaticSpellingCorrectionEnabled = false;
        NSNavPanelExpandedStateForSaveMode = true;
        NSNavPanelExpandedStateForSaveMode2 = true;
        "com.apple.trackpad.scaling" = 3.0;
        "com.apple.mouse.tapBehavior" = 1;
        "com.apple.swipescrolldirection" = false;
      };

      dock = {
        autohide = true;
        mru-spaces = false;
        orientation = "bottom";
        show-recents = false;
        tilesize = 48;
        minimize-to-application = true;
        expose-group-apps = true;
      };

      finder = {
        AppleShowAllExtensions = true;
        FXDefaultSearchScope = "SCcf";
        FXEnableExtensionChangeWarning = false;
        ShowPathbar = true;
        ShowStatusBar = true;
        _FXShowPosixPathInTitle = true;
        _FXSortFoldersFirst = true;
        CreateDesktop = false;
      };

      trackpad = {
        Clicking = true;
        TrackpadThreeFingerDrag = true;
        TrackpadRightClick = true;
      };

      screencapture = {
        location = "~/Pictures/screenshots";
        type = "png";
        disable-shadow = true;
      };

      screensaver = {
        askForPassword = true;
        askForPasswordDelay = 0;
      };

      SoftwareUpdate = {
        AutomaticallyInstallMacOSUpdates = true;
      };

      menuExtraClock = {
        ShowSeconds = true;
        ShowDayOfWeek = true;
        ShowDate = 2;
      };

      ActivityMonitor = {
        IconType = 5;
        ShowCategory = 0;
        SortColumn = "CPUUsage";
        SortDirection = 0;
      };

      LaunchServices = {
        LSQuarantine = false;
      };

      CustomUserPreferences = {
        "com.apple.desktopservices" = {
          DSDontWriteNetworkStores = true;
          DSDontWriteUSBStores = true;
        };
        "com.apple.finder" = {
          ShowExternalHardDrivesOnDesktop = false;
          ShowHardDrivesOnDesktop = false;
          ShowMountedServersOnDesktop = false;
          ShowRemovableMediaOnDesktop = false;
        };
        "com.apple.TimeMachine" = {
          DoNotOfferNewDisksForBackup = true;
        };
      };
    };

    keyboard = {
      enableKeyMapping = true;
      remapCapsLockToEscape = true;
    };
  };
}
