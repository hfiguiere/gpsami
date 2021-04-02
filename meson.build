project('gpsami',
          version: '0.2.0',
    meson_version: '>= 0.50.0',
  default_options: [ 'warning_level=2',
                   ],
)

i18n = import('i18n')

application_id = 'net.figuiere.gpsami'

datadir = get_option('prefix') / get_option('datadir')
iconsdir = datadir / 'icons'

subdir('data')
subdir('src')
subdir('po')

meson.add_install_script('build-aux/meson/postinstall.py')
