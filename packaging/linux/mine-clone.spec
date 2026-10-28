# Packages an already built release binary; build it with packaging/linux/build-rpm.sh.
%global debug_package %{nil}
%global __strip /bin/true
%define _build_id_links none

Name:           mine-clone
Version:        %{game_version}
Release:        %{game_release}%{?dist}
Summary:        A Minecraft-like block game written in Rust
License:        Proprietary
URL:            https://github.com/Githyot/mine-clone
ExclusiveArch:  x86_64

# Loaded at runtime with dlopen, so rpm cannot find them on its own.
Requires:       vulkan-loader
Requires:       libxkbcommon
Requires:       libxkbcommon-x11
Requires:       libX11
Requires:       libXcursor
Requires:       libXi
Requires:       libXrandr
Requires:       libwayland-client
Requires:       libwayland-cursor
Recommends:     mesa-vulkan-drivers

%description
Mine Clone is a Minecraft-like block game written in Rust and rendered with
wgpu on Vulkan. Run it from the applications menu or with "mine-clone".
Saves, settings and resource packs live in ~/.local/share/mine-clone.

%install
mkdir -p %{buildroot}/opt/mine-clone %{buildroot}%{_bindir} %{buildroot}%{_datadir}/applications
cp -a %{stage}/opt/mine-clone/. %{buildroot}/opt/mine-clone/
install -m 0755 %{stage}/mine-clone.sh %{buildroot}%{_bindir}/mine-clone
install -m 0644 %{stage}/mine-clone.desktop %{buildroot}%{_datadir}/applications/mine-clone.desktop
for s in 32 48 64 128 256 512; do
    install -D -m 0644 %{stage}/icons/icon_$s.png %{buildroot}%{_datadir}/icons/hicolor/${s}x${s}/apps/mine-clone.png
done

%files
/opt/mine-clone
%{_bindir}/mine-clone
%{_datadir}/applications/mine-clone.desktop
%{_datadir}/icons/hicolor/*/apps/mine-clone.png
