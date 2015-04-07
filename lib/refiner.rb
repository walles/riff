require 'matrix'
require 'diff_string'

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
class Refiner
  include Colors

  attr_reader :refined_old
  attr_reader :refined_new

  def initialize(old, new)
    @old = old.chomp
    @new = new.chomp

    @refined_old = DiffString.new('-', RED)
    @refined_new = DiffString.new('+', GREEN)

    @matrix = compute_matrix()

    refine(@old.length, @new.length)
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

  def refine(old_index, new_index)
    old_char = @old[old_index - 1]
    new_char = @new[new_index - 1]

    if (old_index > 0 && new_index > 0) && (old_char == new_char)
      refine(old_index - 1, new_index - 1)

      add_context_char(old_char)
    elsif (new_index > 0) &&
          (old_index == 0 || @matrix[old_index, new_index - 1] >=
                             @matrix[old_index - 1, new_index])
      refine(old_index, new_index - 1)

      add_added_char(new_char)
    elsif (old_index > 0) &&
          (new_index == 0 || @matrix[old_index, new_index - 1] <
                             @matrix[old_index - 1, new_index])
      refine(old_index - 1, new_index)

      add_removed_char(old_char)
    end
  end
end
