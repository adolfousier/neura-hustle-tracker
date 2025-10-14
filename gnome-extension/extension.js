import {Extension} from 'resource:///org/gnome/shell/extensions/extension.js';
import Gio from 'gi://Gio';
import * as Main from 'resource:///org/gnome/shell/ui/main.js';

export default class TimetrackerPidExtension extends Extension {
  enable() {
    this._dbusObject = Gio.DBusExportedObject.wrapJSObject(this._interfaceXml, this);
    this._dbusObject.export(Gio.DBus.session, '/com/timetracking/PidGetter');
  }

  disable() {
    if (this._dbusObject) {
      this._dbusObject.unexport();
      this._dbusObject = null;
    }
  }

  GetActiveWindowPid() {
    let focused = global.display.focus_window;
    if (focused) {
      return focused.get_pid();
    }
    return 0;
  }

  get _interfaceXml() {
    return `
<node>
  <interface name="com.timetracking.PidGetter">
    <method name="GetActiveWindowPid">
      <arg type="u" direction="out" name="pid"/>
    </method>
  </interface>
</node>
`;
  }
}