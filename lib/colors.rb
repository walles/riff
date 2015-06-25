# ANSI Color related escape code constants
module Colors
  ESC = 27.chr

  BOLD = "#{ESC}[1m"
  CYAN = "#{ESC}[36m"
  GREEN = "#{ESC}[32m"
  RED = "#{ESC}[31m"
  DEFAULT_COLOR = "#{ESC}[39m"

  REVERSE = "#{ESC}[7m"
  NOT_REVERSE = "#{ESC}[27m"

  RESET = "#{ESC}[m"

  def reversed(string)
    return "#{REVERSE}#{string}#{NOT_REVERSE}"
  end

  def uncolor(string)
    return string.gsub(/#{ESC}[^m]*m/, '')
  end
end
