#
# Copyright 2024 YOUR NAME
#
# All Rights Reserved.

name "nodex-agent"
maintainer "CHANGE ME"
homepage "https://CHANGE-ME.com"

# Defaults to C:/nodex-agent on Windows
# and /opt/nodex-agent on all other platforms
install_dir "#{default_root}/#{name}"
# skip_health_checks true

build_version Omnibus::BuildVersion.semver
build_iteration 1

# Creates required build directories
dependency "preparation"
dependency "init-scripts"
dependency "build-nodex-agent"

exclude "**/.git"
exclude "**/bundler/git"

package :deb do
  compression_level 9
  # signed false
end

package :rpm do
  compression_level 9
  # signed false
end

if ENV['TARGET_PLATFORM'] == 'ubuntu'
  package_scripts_path "#{Omnibus::Config.project_root}/package-scripts/nodex-agent-deb"
end
# else
#   package_scripts_path "#{Omnibus::Config.project_root}/package-scripts/nodex-agent-rpm"
# elsif osx_target?
#     package_scripts_path "#{Omnibus::Config.project_root}/package-scripts/nodex-agent-dmg"
