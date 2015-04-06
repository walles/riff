require 'matrix'
require 'diff_string'

FIXME: write tests for this class

# Inspired by
# http://stackoverflow.com/questions/12683772/how-to-modify-a-matrix-ruby-std-lib-matrix-class
class Matrix
  public :"[]=", :set_element, :set_component
end

# Print longest common substring based diff between two strings.
#
# The diff format is first the old string:
# * in red
# * with each line prefixed with minuses
# * removed characters highlighted in inverse video
#
# Then comes the new string:
# * in green
# * with each line prefixed with plusses
# * added characters highlighted in inverse video
#
# The algorithm is inspired by:
# https://en.wikipedia.org/wiki/Longest_common_subsequence_problem#Print_the_diff
class LongestCommonSubstring
  def initialize(old, new)
    @old = old.chomp
    @new = new.chomp

    @refined_old = DiffString.new('-', DiffString::RED)
    @refined_new = DiffString.new('+', DiffString::GREEN)

    @matrix = compute_matrix()
  end

  # Inspired by:
  # https://en.wikipedia.org/wiki/Longest_common_subsequence_problem#Computing_the_length_of_the_LCS
  def compute_matrix()
    matrix = Matrix.zero(@old.length + 1, @new.length + 1)

    (1..@old.length).each do |old_index|
      (1..@new.length).each do |new_index|
        if @old[old_index] == @new[new_index]
          matrix[old_index, new_index] =
            matrix[old_index - 1, new_index - 1] + 1
        else
          matrix[old_index, new_index] =
            [
              matrix[old_index, new_index - 1],
              matrix[old_index - 1, new_index]
            ].max()
        end
      end
    end

    return matrix
  end

  def add_context_char(char)
    @refined_old.add_char(char, false)
    @refined_new.add_char(char, false)
  end

  def add_added_char(char)
    @refined_new.add_char(char, true)
  end

  def add_removed_char(char)
    @refined_old.add_char(char, true)
  end

  def print_diff_helper(old_index, new_index)
    old_char = @old[old_index - 1]
    new_char = @new[new_index - 1]

    if (old_index > 0 && new_index > 0) && (old_char == new_char)
      print_diff_helper(old_index - 1, new_index - 1)

      add_context_char(old_char)
    elsif (new_index > 0) &&
          (old_index == 0 || @matrix[old_index, new_index - 1] >=
                             @matrix[old_index - 1, new_index])
      print_diff_helper(old_index, new_index - 1)

      add_added_char(new_char)
    elsif (old_index > 0) &&
          (new_index == 0 || @matrix[old_index, new_index - 1] <
                             @matrix[old_index - 1, new_index])
      print_diff_helper(old_index - 1, new_index)

      add_removed_char(old_char)
    end
  end

  def print_diff()
    print_diff_helper(@old.length, @new.length)

    puts @refined_old
    puts @refined_new
  end
end
