# Methods for finding out the Riff version
module Version
  # FIXME: This method should return null if this command fails
  def git_version
    return `cd #{__dir__} ; git describe --dirty`
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
    raw_git_version = git_version
    unless raw_git_version.nil?
      return semantify_git_version(raw_git_version)
    end

    return rubygems_version
  end
end
