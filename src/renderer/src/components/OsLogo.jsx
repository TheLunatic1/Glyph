import React from 'react';

// Maps OS ID (from /etc/os-release ID= field) to devicons icon name
// https://devicon.dev/
const OS_ICON_MAP = {
  // Debian family
  ubuntu: 'ubuntu',
  debian: 'debian',
  linuxmint: 'debian',
  mint: 'debian',
  kali: 'debian',
  'kali-linux': 'debian',
  raspbian: 'raspberrypi',
  pop: 'ubuntu',
  'pop!_os': 'ubuntu',

  // Red Hat family
  centos: 'centos',
  fedora: 'fedora',
  rhel: 'redhat',
  'red hat': 'redhat',
  rocky: 'centos',
  almalinux: 'centos',
  alma: 'centos',
  oracle: 'centos',

  // SUSE family
  opensuse: 'opensuse',
  'opensuse-leap': 'opensuse',
  'opensuse-tumbleweed': 'opensuse',
  suse: 'opensuse',

  // Arch family
  arch: 'archlinux',
  archlinux: 'archlinux',
  manjaro: 'archlinux',
  endeavouros: 'archlinux',
  garuda: 'archlinux',

  // AWS / Cloud
  amazon: 'amazonwebservices',
  amzn: 'amazonwebservices',

  // Other common distros
  alpine: 'alpine',   // will 404 on devicons, handled by onError
  gentoo: 'gentoo',
  void: 'linux',
  nixos: 'nixos',

  // BSD family (no devicons, will fall through to avatar)
  freebsd: null,
  openbsd: null,
  netbsd: null,

  // macOS (for completeness)
  darwin: 'apple',
  macos: 'apple',

  // Embedded / routers (no devicons, fall through to avatar)
  openwrt: null,
  linux: 'linux',
};

export default function OsLogo({ server, className = 'w-8 h-8' }) {
  if (server.os) {
    const osKey = server.os.toLowerCase().trim();
    const icon = osKey in OS_ICON_MAP ? OS_ICON_MAP[osKey] : osKey;

    // If explicitly mapped to null (e.g. openwrt, freebsd), skip to avatar
    if (icon === null) {
      return <OsAvatar name={server.os.toUpperCase()} serverName={server.name} className={className} />;
    }

    return (
      <img
        src={`https://cdn.jsdelivr.net/gh/devicons/devicon/icons/${icon}/${icon}-original.svg`}
        alt={server.os}
        onError={(e) => {
          // Try -plain variant
          const plain = `https://cdn.jsdelivr.net/gh/devicons/devicon/icons/${icon}/${icon}-plain.svg`;
          e.target.onerror = () => {
            // Final fallback: ui-avatars
            e.target.onerror = null;
            e.target.src = avatarUrl(server.name);
          };
          e.target.src = plain;
        }}
        className={`${className} object-contain`}
      />
    );
  }

  // No OS detected yet — show avatar
  return (
    <img
      src={avatarUrl(server.name)}
      alt={server.name}
      className={`${className} rounded-full object-cover`}
    />
  );
}

// Coloured letter badge for OSes with no icon (OpenWrt, FreeBSD, etc.)
function OsAvatar({ name, serverName, className }) {
  // Use the first 2 letters of the OS name, not the server name
  const initials = name.replace(/[^A-Z0-9]/gi, '').slice(0, 2).toUpperCase() || '??';
  return (
    <img
      src={avatarUrl(name, initials)}
      alt={name}
      className={`${className} rounded-full object-cover`}
    />
  );
}

function avatarUrl(label, override) {
  const text = override || label;
  return `https://ui-avatars.com/api/?name=${encodeURIComponent(text)}&background=262a40&color=818cf8&size=48&bold=true`;
}
