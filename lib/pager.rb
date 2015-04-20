module Pager
  def page(text)
    if $stdout.isatty
      IO.popen('LESS=FRX less', 'w') { |pager| pager.print text }
    else
      print text
    end
  end
end
