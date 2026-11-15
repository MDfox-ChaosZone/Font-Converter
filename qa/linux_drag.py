import sys
from pathlib import Path
import gi
gi.require_version("Gtk", "3.0")
from gi.repository import Gtk, Gdk
window = Gtk.Window(title="Font QA drag source")
window.set_default_size(160, 90)
button = Gtk.Button(label="Drag font paths")
button.drag_source_set(Gdk.ModifierType.BUTTON1_MASK, [], Gdk.DragAction.COPY)
button.drag_source_add_uri_targets()
def provide(widget, context, selection, info, when):
    uris = [Path(path).resolve().as_uri() for path in sys.argv[1:]]
    print("GTK supplied", uris, flush=True)
    selection.set_uris(uris)
button.connect("drag-data-get", provide)
window.add(button)
window.show_all()
Gtk.main()
