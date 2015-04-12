# coding: utf-8
require 'colors'

# An added or removed part of a diff, which can contain both
# highlighted and not highlighted characters. For multi line strings,
# each line will be prefixed with prefix and color.
class DiffString
  include Colors

  def initialize(prefix, color)
    @reverse = false
    @prefix = prefix
    @color = color
    @string = prefix
  end

  def add(string, reverse)
    if reverse != @reverse
      @string += reverse ? REVERSE : NOT_REVERSE
    end
    @reverse = reverse

    if @string.end_with? "\n"
      @string += @color
      @string += @prefix
    end

    @string += string
  end

  def to_s()
    string = @string
    newline = ''
    if string.end_with? "\n"
      string.chomp!
      newline = "\n"
    end

    suffix = @color.empty? ? '' : RESET
    return @color + string + suffix + newline
  end
end
