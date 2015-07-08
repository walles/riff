# Whitespace error linter
module WhitespaceLint
  def add_line_highlights(line, base_index, highlights)
    last_seen_whitespace = -1
    last_seen_non_ws = -1
    line.each_char.with_index do |char, index|
      break if char == "\n"
      break if char == "\r"

      if char == ' ' || char == "\t"
        last_seen_whitespace = index
      else
        last_seen_non_ws = index
      end
    end

    if last_seen_non_ws < last_seen_whitespace
      ((last_seen_non_ws + 1)..last_seen_whitespace).each do |index|
        highlights << (base_index + index)
      end
    end
  end

  def collect_ws_highlights(string)
    highlights = Set.new()
    line_start_index = 0
    string.each_line do |line|
      add_line_highlights(line, line_start_index, highlights)

      line_start_index += line.length
    end
    return highlights
  end
end
