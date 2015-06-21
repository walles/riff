# coding: utf-8
require 'colors'

# An added or removed part of a diff, which can contain both
# highlighted and not highlighted characters. For multi line strings,
# each line will be prefixed with prefix and color.
class DiffString
  include Colors

  # Note that the color argument can be the empty string
  def initialize(prefix, color)
    @prefix = prefix
    @base_color = color

    @string = ''

    @reverse = false
    @color = @base_color
  end

  # rubocop:disable Metrics/CyclomaticComplexity
  # rubocop:disable Metrics/PerceivedComplexity
  def add(string, reverse, color = '')
    color = @base_color if color.empty?

    if reverse && string == "\n"
      add('↵', true)
      add("\n", false)
      return
    end

    if @string.empty?() || @string.end_with?("\n")
      @string += @base_color
      @string += @prefix
    end

    if reverse != @reverse
      @string += reverse ? REVERSE : NOT_REVERSE
    end
    @reverse = reverse

    if color != @color
      @string += color
    end
    @color = color

    @string += string
  end

  def to_s()
    return '' if @string.empty?

    string = @string
    string.chomp! if string.end_with? "\n"

    suffix = @base_color.empty? ? '' : RESET
    return string + suffix + "\n"
  end

  def self.decorate_string(prefix, color, string)
    decorated = DiffString.new(prefix, color)
    decorated.add(string, false)
    return decorated.to_s
  end
end
