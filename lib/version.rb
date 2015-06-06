require 'English'

# Methods for finding out the Riff version
module Version
  def git_version(dirty)
    dirty_flag = dirty ? '--dirty' : ''
    version = `cd #{__dir__} ; git describe #{dirty_flag} 2> /dev/null`.chomp
    if $CHILD_STATUS.success?
      return version
    else
      return nil
    end
  end

  def rubygems_version
    return Gem::Specification.find_by_name('riffdiff').version.to_s
  end

  # Turn git describe output into a semantic version, inspired by riff.gemspec
  def semantify_git_version(raw)
    return nil if raw.nil?

    return raw if raw.end_with? '-dirty'

    return (raw + '.0') if /^[0-9]+\.[0-9]+$/.match(raw)

    if /^([0-9]+\.[0-9]+)-([0-9]+)-/.match(raw)
      return "#{$1}.#{$2}"
    end

    return raw
  end

  def version
    semantic_git_version = semantify_git_version(git_version(false))
    return semantic_git_version unless semantic_git_version.nil?

    return rubygems_version
  end

  def semantic_version
    return semantify_git_version(git_version(false))
  end

  def dirty?
    return git_version(true).include?('-dirty')
  end
end
