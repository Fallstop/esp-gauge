!macro NSIS_HOOK_POSTINSTALL
  CreateShortcut "$SMPROGRAMS\ESP Gauge USB driver setup.lnk" "$INSTDIR\drivers\CH341SER.EXE"

  ; The vendor installer is interactive; never interrupt unattended app updates.
  ${If} $UpdateMode <> 1
  ${AndIf} $PassiveMode <> 1
  ${AndIfNot} ${Silent}
    MessageBox MB_YESNO|MB_ICONQUESTION "Install the USB driver for your ESP Gauge board now?$\r$\n$\r$\nRecommended for first-time setup. Windows will ask for administrator approval, then the WCH installer will open. Choose INSTALL in that window.$\r$\n$\r$\nIf your board already connects, you can choose No. You can run ESP Gauge USB driver setup from the Start menu later." IDNO esp_gauge_driver_done

    DetailPrint "Opening WCH USB driver setup..."
    ClearErrors
    ExecShellWait "runas" "$INSTDIR\drivers\CH341SER.EXE"
    ${If} ${Errors}
      MessageBox MB_OK|MB_ICONEXCLAMATION "USB driver setup could not start or administrator approval was cancelled.$\r$\n$\r$\nESP Gauge is installed. To connect your board, run ESP Gauge USB driver setup from the Start menu when you have administrator access."
      ClearErrors
    ${EndIf}
  ${EndIf}
  esp_gauge_driver_done:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Windows owns the shared driver; other CH340 devices may still need it.
  Delete "$SMPROGRAMS\ESP Gauge USB driver setup.lnk"
!macroend
