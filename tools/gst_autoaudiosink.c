/* Stand-in for gst-plugins-good autoaudiosink. Do not register deinterleave
   or scaletempo — decodebin will autoplug them and ogg demux fails not-linked. */

#define PACKAGE "blighnet"
#define VERSION "1.0"

#include <gst/gst.h>

typedef struct {
  GstBin parent;
} GstBlighnetAudioSink;
typedef struct {
  GstBinClass parent_class;
} GstBlighnetAudioSinkClass;

G_DEFINE_TYPE(GstBlighnetAudioSink, gst_blighnet_audio_sink, GST_TYPE_BIN)

static GstElement *make_real_sink(void) {
  GstElement *sink = gst_element_factory_make("pipewiresink", "real-sink");
  if (!sink)
    sink = gst_element_factory_make("alsasink", "real-sink");
  if (!sink)
    return NULL;

  /* WebAudio timestamps are not a movie clock. sync=TRUE drops the stream. */
  g_object_set(sink, "sync", FALSE, NULL);
  if (g_object_class_find_property(G_OBJECT_GET_CLASS(sink), "async"))
    g_object_set(sink, "async", TRUE, NULL);

  if (g_object_class_find_property(G_OBJECT_GET_CLASS(sink), "stream-properties")) {
    GstStructure *props = gst_structure_new(
        "props",
        "media.role", G_TYPE_STRING, "Music",
        "media.type", G_TYPE_STRING, "Audio",
        "media.category", G_TYPE_STRING, "Playback",
        "application.name", G_TYPE_STRING, "Blightnet",
        "node.description", G_TYPE_STRING, "Blightnet",
        NULL);
    g_object_set(sink, "stream-properties", props, NULL);
    gst_structure_free(props);
  }

  if (g_object_class_find_property(G_OBJECT_GET_CLASS(sink), "target-object")) {
    const gchar *target = g_getenv("BLIGHNET_AUDIO_TARGET");
    if (target && target[0])
      g_object_set(sink, "target-object", target, NULL);
  }
  return sink;
}

static void gst_blighnet_audio_sink_init(GstBlighnetAudioSink *self) {
  GstElement *sink = make_real_sink();
  GstPad *pad;
  GstPad *ghost;
  if (!sink)
    return;
  gst_bin_add(GST_BIN(self), sink);
  pad = gst_element_get_static_pad(sink, "sink");
  if (!pad)
    return;
  ghost = gst_ghost_pad_new("sink", pad);
  gst_object_unref(pad);
  gst_pad_set_active(ghost, TRUE);
  gst_element_add_pad(GST_ELEMENT(self), ghost);
}

static void gst_blighnet_audio_sink_class_init(GstBlighnetAudioSinkClass *klass) {
  GstElementClass *element_class = GST_ELEMENT_CLASS(klass);
  static GstStaticPadTemplate sink_tmpl = GST_STATIC_PAD_TEMPLATE(
      "sink", GST_PAD_SINK, GST_PAD_ALWAYS, GST_STATIC_CAPS("ANY"));
  gst_element_class_set_static_metadata(
      element_class, "Auto audio sink", "Sink/Audio",
      "Blightnet fallback autoaudiosink (pipewire/alsa)", "Blightnet");
  gst_element_class_add_static_pad_template(element_class, &sink_tmpl);
}

static gboolean plugin_init(GstPlugin *plugin) {
  return gst_element_register(plugin, "autoaudiosink", GST_RANK_PRIMARY, gst_blighnet_audio_sink_get_type());
}

GST_PLUGIN_DEFINE(
    GST_VERSION_MAJOR,
    GST_VERSION_MINOR,
    blighnetaudio,
    "Blightnet WebKit audio fallbacks",
    plugin_init,
    "1.0",
    "LGPL",
    "blighnet",
    "http://127.0.0.1:8765/")
