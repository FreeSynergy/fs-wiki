# Scenario of the typing test (fs-release actions/typing-test): defines focus(), which brings a text field of the window
# into focus. The window opens on the list of pages (Admin frontend); its search field takes the text. Positions are
# relative to the window's top left corner (WX0, WY0, set by the test; title bar included), for the window size of the
# workflow (`--size 1000x640`): the field is about 44 px high at y 52..96, y=88 hits it with and without a title bar (as
# in fs-people and fs-groups, same frontend). The window starts its own server (empty wiki) in the test's private folders.
focus() {
  local x=$((WX0 + 170)) y=$((WY0 + 88))
  TT_NEAR="$x,$y"; export TT_NEAR
  c "move $x $y" 0.4
  c "click $x $y" 1
}
