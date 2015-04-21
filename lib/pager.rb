# Add a pager() method that can send text to a pager.
#
# With "pager" referring to something like less or moar.
module Pager
  def less(text)
    IO.popen('LESS=FRX less', 'w') { |pager| pager.print text }
  end

  def page(text)
    if !$stdout.isatty
      print text
    else
      less(text)
    end
  end
end
