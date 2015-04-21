# Add a pager() method that can send text to a pager.
#
# With "pager" referring to something like less or moar.
#
# This file attempts to be as close as possible to what Git is
# doing. For reference, do "git help config", search for "core.pager"
# and compare that text to the page() method below.
module Pager
  # Checking for this variable before launching $PAGER should prevent
  # us from fork bombing if somebody sets the PAGER environment
  # variable to point to us.
  DONT_USE_PAGER_VAR = '_RIFF_PREVENT_PAGING_LOOP'

  def pipe_text_into_command(text, command)
    env = {
      DONT_USE_PAGER_VAR => '1'
    }

    # Set LESS=FRX unless $LESS already has a value
    env['LESS'] = 'FRX' unless ENV['LESS']

    # Set LV=-c unless $LV already has a value
    env['LV'] = '-c' unless ENV['LV']

    IO.popen(env, command, 'w') { |pager| pager.print text }
  end

  def less(text)
    pipe_text_into_command(text, 'less')
  end

  # If $DONT_USE_PAGER_VAR is set, we shouldn't use $PAGER
  def dont_use_pager?
    return true if ENV[DONT_USE_PAGER_VAR]
    return true if ENV['PAGER'].nil?
    return true if ENV['PAGER'].empty?
    return false
  end

  def page(text)
    if !$stdout.isatty
      print text
    elsif dont_use_pager?
      less(text)
    elsif ENV['PAGER']
      pipe_text_into_command(text, ENV['PAGER'])
    else
      less(text)
    end
  end
end
