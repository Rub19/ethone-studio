#!/usr/bin/perl
# Wraps literal interface strings in tl("…") so they can be translated (see crates/studio-ui/src/i18n.rs).
# usage: perl scripts/i18n-wrap.pl <import path, e.g. studio_ui::i18n::tl or crate::i18n::tl> <files…>
# Only plain literals are wrapped (no {} placeholders); already wrapped ones are left alone.
use strict; use warnings;
my $import = shift or die "usage: i18n-wrap.pl <import> <files>\n";
my $L = qr/"((?:[^"\\{}]|\\.)*[A-Za-zÀ-ÿ](?:[^"\\{}]|\\.)*)"/;
my $total = 0;
for my $f (@ARGV) {
  open my $in, '<:utf8', $f or die "$f: $!"; local $/; my $s = <$in>; close $in;
  my $n = 0;
  $n += $s =~ s/RichText::new\($L\)/RichText::new(tl("$1"))/g;
  $n += $s =~ s/\.(button|label|small_button|heading|hint_text|on_hover_text|on_disabled_hover_text)\($L\)/.$1(tl("$2"))/g;
  $n += $s =~ s/\.(menu_button|collapsing)\($L,/.$1(tl("$2"),/g;
  $n += $s =~ s/\.(selectable_label|checkbox|radio)\(([^;\n]*?), $L\)/.$1($2, tl("$3"))/g;
  $n += $s =~ s/\.selectable_value\(([^;\n]*?), $L\)/.selectable_value($1, tl("$2"))/g;
  $n += $s =~ s/pro::(primary|secondary|ghost|chip|pill|tip|primary_icon|secondary_icon|label_value|option_slider)\(ui, $L/pro::$1(ui, tl("$2")/g;
  $n += $s =~ s/dock_panel\(ui, ("[^"]*"), $L/dock_panel(ui, $1, tl("$2")/g;
  $n += $s =~ s/\b(pill|cta)\(ui, $L/$1(ui, tl("$2")/g;
  $n += $s =~ s/nav_row\(ui, ([^,]+), ([^,]+), $L/nav_row(ui, $1, $2, tl("$3")/g;
  $n += $s =~ s/pref_title\(ui, $L\)/pref_title(ui, tl("$1"))/g;
  $n += $s =~ s/Cmd::new\($L, $L/Cmd::new(tl("$1"), tl("$2")/g;
  $n += $s =~ s{(segmented\([^;\n]*?&\[)([^\]\n]*)(\])}{ my ($a, $b, $c) = ($1, $2, $3); my $k = ($b =~ s/(?<!tl\()$L/tl("$1")/g) || 0; $a . $b . $c }ge;
  next unless $n;
  unless ($s =~ /use \Q$import\E;/) {
    $s =~ s/^(use [^\n]*;\n)/$1use $import;\n/m or $s = "use $import;\n" . $s;
  }
  open my $out, '>:utf8', $f or die; print $out $s; close $out;
  printf "%-45s %4d\n", $f, $n;
  $total += $n;
}
print "total wrapped: $total\n";
