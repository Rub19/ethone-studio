#!/usr/bin/perl
# Collects every tl("…") string of the given sources into a FR<TAB>EN dictionary, keeping the
# translations already there. Untranslated entries get an empty English column.
# usage: perl scripts/i18n-extract.pl <dictionary.tsv> <files…>
use strict; use warnings; use utf8;
my $dict = shift or die "usage: i18n-extract.pl <dict.tsv> <files>\n";
my (%en, @order, %seen);
my @head;
if (open my $d, '<:utf8', $dict) {
  while (my $l = <$d>) {
    chomp $l; $l =~ s/\r$//;
    if ($l =~ /^#/) { push @head, $l; next }
    my ($fr, $e) = split /\t/, $l, 2;
    next unless defined $fr && length $fr;
    $en{$fr} = $e // '';
    push @order, $fr unless $seen{$fr}++;
  }
}
my $new = 0;
for my $f (@ARGV) {
  open my $in, '<:utf8', $f or die "$f: $!"; local $/; my $s = <$in>; close $in;
  while ($s =~ /\btl\("((?:[^"\\]|\\.)*)"\)/g) {
    (my $fr = $1) =~ s/\\"/"/g;
    $fr =~ s/\\n/ /g;
    next if $seen{$fr}++;
    push @order, $fr; $en{$fr} = ''; $new++;
  }
}
for my $f (@ARGV) { open my $in, "<:utf8", $f or die; local $/; my $s = <$in>; close $in; while ($s =~ /\(Tool::\w+, "([^"\\]+)"/g) { my $fr = $1; next if $seen{$fr}++; push @order, $fr; $en{$fr} = ""; $new++ } }
for my $f (@ARGV) { open my $in, "<:utf8", $f or die; local $/; my $s = <$in>; close $in; while ($s =~ /[\[(,]\s*\(\s*"([^"\\]*?(?:[A-ZÀ-Ý][^"\\]*\s|[àâçéèêëîïôûùœ])[^"\\]*)"\s*,/g) { my $fr = $1; next if $seen{$fr}++; push @order, $fr; $en{$fr} = ""; $new++ } }
for my $f (@ARGV) { open my $in, "<:utf8", $f or die; local $/; my $s = <$in>; close $in; while ($s =~ /[\[(,]\s*"([A-ZÀ-Ý][^"\\{}:\/]{0,58}[a-zà-ÿ][^"\\{}:\/]{0,58})"(?=\s*[,)\]])/g) { my $fr = $1; next if $fr =~ /^[A-Z][a-z]*[A-Z]|\.(?:rs|png|jpg|json)$/ || $seen{$fr}++; push @order, $fr; $en{$fr} = ""; $new++ } }
for my $f (@ARGV) { open my $in, "<:utf8", $f or die; local $/; my $s = <$in>; close $in; while ($s =~ /(?:\b(?:name|hint|label|title|desc|tip|help):\s*|=>\s*|\b[a-z_]+\()"([A-ZÀ-Ý][^"\\{}]*[a-zà-ÿ][^"\\{}]*)"/g) { my $fr = $1; next if $fr =~ /^[A-Z][a-z]*[A-Z]|\.(?:rs|png|jpg|json)$|^[A-Z][a-z]+::/ || $seen{$fr}++; push @order, $fr; $en{$fr} = ""; $new++ } }
for my $f (@ARGV) { open my $in, "<:utf8", $f or die; local $/; my $s = <$in>; close $in; my @found; while ($s =~ /\(\s*"[^"\\]*"\s*,\s*"([^"\\]*?[a-zà-ÿ][^"\\]*?(?:\s|[àâçéèêëîïôûùœ])[^"\\]*)"\s*[,)]/g) { push @found, $1 } while ($s =~ /\[&(?:'static )?str; \d+\] = \[([^\]]*)\]/g) { my $list = $1; push @found, $1 while $list =~ /"([^"\\]*[A-Za-zÀ-ÿ][^"\\]*)"/g } for my $fr (@found) { next if $fr =~ /[{}]/ || $seen{$fr}++; push @order, $fr; $en{$fr} = ""; $new++ } }

@head = ("# French text<TAB>English text. One line per text; an empty English column keeps the French.") unless @head;
open my $out, '>:utf8', $dict or die; print $out join("\n", @head), "\n";
print $out "$_\t$en{$_}\n" for @order;
close $out;
my $missing = grep { $en{$_} eq '' } @order;
print "$dict: ", scalar(@order), " entries, $new new, $missing to translate\n";
