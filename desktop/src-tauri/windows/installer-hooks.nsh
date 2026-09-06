Var EspGaugeDriverStatus

!macro ESP_GAUGE_CHECK_DRIVER
  Push $1
  nsExec::ExecToStack /TIMEOUT=10000 '"$INSTDIR\${MAINBINARYNAME}.exe" --check-usb-driver'
  Pop $EspGaugeDriverStatus
  Pop $1
  Pop $1
!macroend

!macro NSIS_HOOK_POSTINSTALL
  Delete "$SMPROGRAMS\ESP Gauge USB driver setup.lnk"
  !insertmacro ESP_GAUGE_CHECK_DRIVER

  ; The vendor installer is interactive; never interrupt unattended app updates.
  ${If} $EspGaugeDriverStatus != 0
  ${AndIf} $UpdateMode <> 1
  ${AndIf} $PassiveMode <> 1
  ${AndIfNot} ${Silent}
    MessageBox MB_YESNO|MB_ICONQUESTION "Install the USB driver for your ESP Gauge board now?$\r$\n$\r$\nWindows will ask for administrator approval, then the WCH installer will open. Choose INSTALL in that window.$\r$\n$\r$\nYou can also open USB driver setup from ESP Gauge's Connect board page later." IDNO esp_gauge_driver_done

    DetailPrint "Opening WCH USB driver setup..."
    ClearErrors
    ExecShellWait "runas" "$INSTDIR\drivers\CH341SER.EXE"
    ${If} ${Errors}
      MessageBox MB_OK|MB_ICONEXCLAMATION "USB driver setup could not start or administrator approval was cancelled.$\r$\n$\r$\nESP Gauge is installed. Open USB driver setup from its Connect board page when you have administrator access."
      ClearErrors
    ${EndIf}
  ${EndIf}
  esp_gauge_driver_done:
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; Windows owns the shared driver; other CH340 devices may still need it.
  Delete "$SMPROGRAMS\ESP Gauge USB driver setup.lnk"
!macroend
