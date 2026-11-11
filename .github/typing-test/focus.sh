# Scenario of the typing test (fs-release actions/typing-test): defines focus(), which brings a text field of the window
# into focus. The window opens on the list of pages (Admin frontend, empty wiki); "Create" opens the form of a new page and
# its first field ("Path") takes the text. Positions are relative to the window's top left corner (WX0, WY0, set by the
# test; title bar included), for the window size of the workflow (`--size 1000x640`). The window starts its own server in
# the test's private folders.
focus() {
  c "move $((WX0 + 286)) $((WY0 + 178))" 0.4
  c "click $((WX0 + 286)) $((WY0 + 178))" 1.5
  shot after-create
  local x=$((WX0 + 600)) y=$((WY0 + 261))
  TT_NEAR="$x,$y"; export TT_NEAR
  c "move $x $y" 0.4
  c "click $x $y" 1
}
