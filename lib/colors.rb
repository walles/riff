# ANSI Color related escape code constants
module Colors
  ESC = 27.chr

  BOLD = "#{ESC}[1m"
  CYAN = "#{ESC}[36m"
  GREEN = "#{ESC}[32m"
  RED = "#{ESC}[31m"

  REVERSE = "#{ESC}[7m"
  NORMAL = "#{ESC}[27m"

  RESET = "#{ESC}[m"
end
