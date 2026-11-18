#!/usr/bin/perl
# Writes i18n/<code>.tsv for every app from numbered translations of the English texts.
# usage: perl scripts/i18n-langs.pl <english list, one text per line> <code1,code2> <N<TAB>text1<TAB>text2 files…>
# Line N of the English list is translated by line "N<TAB>…" of the translation files.
use strict; use warnings; use utf8;
binmode STDOUT, ':utf8';
my ($list, $codes, @files) = @ARGV;
die "usage: i18n-langs.pl <english list> <codes> <files>\n" unless $list && $codes && @files;
my @codes = split /,/, $codes;
open my $in, '<:utf8', $list or die "$list: $!";
my @en = map { s/\r?\n$//r } <$in>;
close $in;
my %tr;
for my $f (@files) {
  open my $t, '<:utf8', $f or die "$f: $!";
  while (<$t>) {
    s/\r?\n$//;
    my ($n, @cols) = split /\t/, $_, -1;
    next unless defined $n && $n =~ /^\d+$/;
    die "$f: line $n has " . scalar(@cols) . " columns, expected " . scalar(@codes) . "\n" if @cols != @codes;
    $tr{$en[$n - 1]} = \@cols;
  }
}
my @missing = grep { !$tr{$en[$_ - 1]} } 1 .. @en;
die "missing translations for lines: @missing[0 .. ($#missing < 20 ? $#missing : 19)]\n" if @missing;
for my $dir (glob('apps/*/i18n'), 'crates/studio-ui/i18n') {
  my $src = "$dir/en.tsv";
  next unless -f $src;
  open my $d, '<:utf8', $src or die;
  my @pairs;
  while (<$d>) {
    s/\r?\n$//;
    next if /^#/;
    my ($fr, $e) = split /\t/, $_, 2;
    next unless defined $e && $e ne '' && $tr{$e};
    push @pairs, [$fr, $tr{$e}];
  }
  close $d;
  for my $k (0 .. $#codes) {
    open my $o, '>:utf8', "$dir/$codes[$k].tsv" or die;
    print $o "# French text<TAB>translation. One line per text; a missing line falls back to English.\n";
    print $o "$_->[0]\t$_->[1][$k]\n" for @pairs;
    close $o;
  }
  printf "%-28s %4d entries\n", $dir, scalar @pairs;
}
