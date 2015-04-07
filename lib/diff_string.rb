# coding: utf-8
require 'colors'

# An added or removed part of a diff, which can contain both
# highlighted and not highlighted characters.
class DiffString
  include Colors

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
