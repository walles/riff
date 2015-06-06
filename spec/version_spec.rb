# coding: utf-8
require 'version'

include Version

RSpec.describe Version, '#semantify_git_version' do
  context 'nil version' do
    it 'returns nil' do
      expect(semantify_git_version(nil)).to eq(nil)
    end
  end

  context 'tagged version' do
    it 'returns tag.0' do
      expect(semantify_git_version('1.2')).to eq('1.2.0')
    end
  end

  context 'tagged dirty version' do
    it 'returns the raw version string' do
      expect(semantify_git_version('1.2-dirty')).to eq('1.2-dirty')
    end
  end

  context 'non tagged version' do
    it 'returns tag.tagged-commits-ago' do
      expect(semantify_git_version('1.2-5-g695a668')).to eq('1.2.5')
    end
  end

  context 'non tagged dirty version' do
    it 'returns the raw version string' do
      expect(semantify_git_version('1.2-5-g695a668-dirty')).to(
        eq('1.2-5-g695a668-dirty'))
    end
  end
end

RSpec.describe Version, '#git_version' do
  it "doesn't end in a newline" do
    expect(git_version(false)).to(eq(git_version(false).chomp))
  end
end
