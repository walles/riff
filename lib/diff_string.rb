# coding: utf-8

# An added or removed part of a diff, which can contain both
# highlighted and not highlighted characters.
class DiffString
  ESC = 27.chr

  RED = "#{ESC}[31m"
  GREEN = "#{ESC}[32m"

  REVERSE = "#{ESC}[7m"
  NORMAL = "#{ESC}[27m"

  RESET = "#{ESC}[m"

  def initialize(prefix, color)
    @reverse = false
    @prefix = prefix
    @color = color
    @string = prefix
  end

  def add_char(char, reverse)
    if reverse != @reverse
      @string += reverse ? REVERSE : NORMAL
    end
    @reverse = reverse

    @string += char

    if char == "\n"
      @string += @prefix
    end
  end

  def to_s()
    suffix = @color.empty? ? '' : RESET
    return @color + @string + suffix
  end
end
